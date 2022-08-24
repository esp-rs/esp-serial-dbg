use std::{
    marker::PhantomData,
    net::{TcpListener, TcpStream},
};

use gdbstub::{
    arch::{Arch, Registers},
    common::Signal,
    conn::ConnectionExt,
    stub::{run_blocking, GdbStub, SingleThreadStopReason},
    target::{
        ext::{
            base::{
                singlethread::{SingleThreadBase, SingleThreadResume, SingleThreadResumeOps},
                BaseOps,
            },
            breakpoints::{HwBreakpoint, SwBreakpoint, WatchKind},
            memory_map::MemoryMap,
        },
        TargetError, TargetResult,
    },
};
use log::{info, trace};

pub mod riscv_esp32c3;
mod riscv_insn_estimator;
pub mod xtensa_esp32;
pub mod xtensa_esp32s2;
pub mod xtensa_esp32s3;
mod xtensa_insn_estimator;

use crate::{
    gdb::{
        riscv_insn_estimator::riscv_insn_estimator, xtensa_insn_estimator::xtensa_insn_estimator,
    },
    DeviceMessage, SerialDebugConnection,
};

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

fn wait_for_tcp(port: u16) -> DynResult<TcpStream> {
    let sockaddr = format!("127.0.0.1:{}", port);
    info!("Waiting for a GDB connection on {:?}...", sockaddr);

    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;
    info!("Debugger connected from {}", addr);

    Ok(stream)
}

pub(crate) fn gdb_main<REGISTERS>(dbg: &SerialDebugConnection) -> DynResult<()>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    let mut trgt = SerialDbgTarget {
        debug_connection: dbg,
        registers: crate::Registers::Riscv(crate::RiscvRegisters::default()),
        hw_breakpoints: Vec::new(),
        sw_breakpoints: Vec::new(),
        stepping: false,
        temporarly_disabled_sw_breakpoints: Vec::new(),
        phantom: PhantomData::default(),
    };

    let connection: Box<dyn ConnectionExt<Error = std::io::Error>> = Box::new(wait_for_tcp(9001)?);

    dbg.break_execution();
    loop {
        match dbg.pending_message() {
            Some(DeviceMessage::HitBreakpoint(data)) => {
                info!("break on connect hit: {:x?}", data);
                trgt.registers = data;
                break;
            }
            _ => (),
        }
    }

    let gdb = GdbStub::new(connection);

    match gdb.run_blocking::<SerialDbgGdbEventLoop<REGISTERS>>(&mut trgt) {
        Ok(disconnect_reason) => {
            info!("Disconnect {:?}", disconnect_reason);
        }
        Err(err) => {
            info!("GDB Err {:?}", err);
        }
    }

    // clear breakpoints and resume
    for bp in trgt.hw_breakpoints.clone().iter() {
        trgt.remove_hw_breakpoint(bp.address, 0).ok();
    }
    for bp in trgt.sw_breakpoints.clone().iter() {
        trgt.remove_sw_breakpoint(bp.address, 0).ok();
    }
    trgt.resume(None).ok();

    Ok(())
}

pub trait EspRegisters {
    fn set_regs(&mut self, regs: crate::Registers);

    fn architecture() -> Option<&'static str>;

    fn memory_map() -> &'static str;

    fn sw_breakpoint_opcode() -> (usize, [u8; 3]);

    fn hw_breakpoint_start() -> u8;

    fn hw_breakpoint_end() -> u8;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EspArch<REGISTERS>
where
    REGISTERS: Registers + EspRegisters,
{
    phantom: PhantomData<REGISTERS>,
}

impl<REGISTERS> Arch for EspArch<REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    type Usize = u32;

    type Registers = REGISTERS;

    type BreakpointKind = usize;

    type RegId = ();

    fn single_step_gdb_behavior() -> gdbstub::arch::SingleStepGdbBehavior {
        gdbstub::arch::SingleStepGdbBehavior::Optional
    }

    fn target_description_xml() -> Option<&'static str> {
        REGISTERS::architecture()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SerialDbgHwBreakpoint {
    id: u8,
    address: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct SerialDbgSwBreakpoint {
    address: u32,
    original_code: [u8; 3],
}

struct SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers + EspRegisters,
{
    debug_connection: &'a SerialDebugConnection,
    registers: crate::Registers,
    hw_breakpoints: Vec<SerialDbgHwBreakpoint>,
    sw_breakpoints: Vec<SerialDbgSwBreakpoint>,
    stepping: bool,
    temporarly_disabled_sw_breakpoints: Vec<SerialDbgSwBreakpoint>,
    phantom: PhantomData<REGISTERS>,
}

impl<'a, REGISTERS> gdbstub::target::Target for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    type Arch = EspArch<REGISTERS>;

    type Error = ();

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }

    fn guard_rail_implicit_sw_breakpoints(&self) -> bool {
        true
    }

    fn support_breakpoints(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }

    fn support_memory_map(
        &mut self,
    ) -> Option<gdbstub::target::ext::memory_map::MemoryMapOps<'_, Self>> {
        Some(self)
    }
}

impl<'a, REGISTERS> MemoryMap for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn memory_map_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        fn copy_to_buf(data: &[u8], buf: &mut [u8]) -> usize {
            let len = buf.len().min(data.len());
            buf[..len].copy_from_slice(&data[..len]);
            len
        }

        fn copy_range_to_buf(data: &[u8], offset: u64, length: usize, buf: &mut [u8]) -> usize {
            let offset = offset as usize;
            if offset > data.len() {
                return 0;
            }

            let start = offset;
            let end = (offset + length).min(data.len());
            copy_to_buf(&data[start..end], buf)
        }

        let memory_map = REGISTERS::memory_map().trim().as_bytes();
        Ok(copy_range_to_buf(memory_map, offset, length, buf))
    }
}

impl<'a, REGISTERS> SingleThreadBase for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> TargetResult<(), Self> {
        regs.set_regs(self.registers);
        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
    ) -> TargetResult<(), Self> {
        println!("TODO write regs {:?}", regs);
        Ok(())
    }

    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
    ) -> TargetResult<(), Self> {
        info!("read addres called {:x} {}", start_addr, data.len());

        if start_addr < 0x3f000000u32 {
            info!("invalid!");
            return Err(TargetError::NonFatal);
        }

        let aligned_start = start_addr & !0b11;
        let offset = (start_addr - aligned_start) as usize;
        let len = data.len() + offset;
        let aligned_len = if len % 4 != 0 {
            len + (4 - (len % 4))
        } else {
            len
        };

        info!("aligned to {:x} {} {}", aligned_start, aligned_len, offset);

        let mut read_data = self
            .debug_connection
            .read_memory(aligned_start, aligned_len as u32);

        // we might have read memory with breakpoint instructions inserted by us!
        // let's fix that
        let end = aligned_start + aligned_len as u32;
        for sw_brkpt in self.sw_breakpoints.iter() {
            if sw_brkpt.address + sw_brkpt.original_code.len() as u32 >= aligned_start
                && sw_brkpt.address <= end
            {
                let rem_len = if sw_brkpt.address >= aligned_start {
                    usize::min(
                        sw_brkpt.original_code.len(),
                        (end - sw_brkpt.address) as usize,
                    )
                } else {
                    sw_brkpt.original_code.len() - (aligned_start - sw_brkpt.address) as usize
                };
                let start_in_original_code =
                    i64::max(0, aligned_start as i64 - sw_brkpt.address as i64) as usize;
                let start_in_data =
                    i64::max(0, sw_brkpt.address as i64 - aligned_start as i64) as usize;
                read_data[start_in_data..][..rem_len]
                    .copy_from_slice(&sw_brkpt.original_code[start_in_original_code..][..rem_len]);
            }
        }
        for sw_brkpt in self.temporarly_disabled_sw_breakpoints.iter() {
            if sw_brkpt.address + sw_brkpt.original_code.len() as u32 >= aligned_start
                && sw_brkpt.address <= end
            {
                let rem_len = if sw_brkpt.address >= aligned_start {
                    usize::min(
                        sw_brkpt.original_code.len(),
                        (end - sw_brkpt.address) as usize,
                    )
                } else {
                    sw_brkpt.original_code.len() - (aligned_start - sw_brkpt.address) as usize
                };
                let start_in_original_code =
                    i64::max(0, aligned_start as i64 - sw_brkpt.address as i64) as usize;
                let start_in_data =
                    i64::max(0, sw_brkpt.address as i64 - aligned_start as i64) as usize;
                read_data[start_in_data..][..rem_len]
                    .copy_from_slice(&sw_brkpt.original_code[start_in_original_code..][..rem_len]);
            }
        }

        data.copy_from_slice(&read_data[offset..][..(data.len())]);
        Ok(())
    }

    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<(), Self> {
        info!("write addres called {:x} {:?}", start_addr, data);

        let aligned_start = start_addr & !0b11;
        let offset = (start_addr - aligned_start) as usize;
        let len = data.len() + offset;
        let aligned_len = if len % 4 != 0 {
            len + (4 - (len % 4))
        } else {
            len
        };

        info!("aligned to {:x} {} {}", aligned_start, aligned_len, offset);

        let mut aligned_data = self
            .debug_connection
            .read_memory(aligned_start, aligned_len as u32);
        aligned_data[offset..][..(data.len())].copy_from_slice(&data);

        info!("writing to 0x{:x} {:x?}", aligned_start, aligned_data);

        self.debug_connection
            .write_memory(aligned_start as u32, aligned_data);

        Ok(())
    }

    // most targets will want to support at resumption as well...
    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<Self>> {
        Some(self)
    }
}

impl<'a, REGISTERS> SingleThreadResume for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn resume(&mut self, _signal: Option<gdbstub::common::Signal>) -> Result<(), Self::Error> {
        info!("resume requested");

        self.stepping = false;
        self.debug_connection.resume();
        Ok(())
    }

    fn support_single_step(
        &mut self,
    ) -> Option<gdbstub::target::ext::base::singlethread::SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl<'a, REGISTERS> gdbstub::target::ext::base::singlethread::SingleThreadSingleStep
    for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn step(&mut self, signal: Option<gdbstub::common::Signal>) -> Result<(), Self::Error> {
        info!(
            "UNSUPPORTED `step` {:?} ... however GDB really wants it so try to emulate it",
            signal
        );
        self.stepping = true;

        // we override two hw-breakpoints
        // and re-enable them when we hit a step breakpoint
        // also remove ALL sw breakpoints and re-enable them
        // when we return form a stepping request
        self.temporarly_disabled_sw_breakpoints = self.sw_breakpoints.clone();
        let to_disable = self.sw_breakpoints.clone();
        for bp in to_disable {
            self.remove_sw_breakpoint(bp.address, 0).ok();
        }
        match self.registers {
            espdbg::Registers::Riscv(regs) => {
                let pc = regs.pc;
                trace!("@ {:08x}", pc);
                let mut data = [0u8; 4];
                self.read_addrs(pc, &mut data).ok();
                let possible_next_pcs = riscv_insn_estimator(data, pc, regs);
                let mut i = REGISTERS::hw_breakpoint_start();
                self.debug_connection.clear_breakpoint(1);
                self.debug_connection.clear_breakpoint(2);
                for addr in possible_next_pcs {
                    trace!("{} ==> {:08x}", i, addr);
                    self.debug_connection.set_breakpoint(addr, i);
                    i += 1;
                }
            }
            espdbg::Registers::Xtensa(regs) => {
                let pc = regs.pc;
                trace!("@ {:08x}", pc);
                let mut data = [0u8; 4];
                self.read_addrs(pc, &mut data).ok();
                let possible_next_pcs = xtensa_insn_estimator(data, pc, regs);
                let mut i = REGISTERS::hw_breakpoint_start();
                self.debug_connection.clear_breakpoint(0);
                self.debug_connection.clear_breakpoint(1);
                for addr in possible_next_pcs {
                    trace!("{} ==> {:08x}", i, addr);
                    self.debug_connection.set_breakpoint(addr, i);
                    i += 1;
                }
            }
        };

        self.debug_connection.resume();
        return Ok(());
    }
}

impl<'a, REGISTERS> gdbstub::target::ext::breakpoints::Breakpoints
    for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    #[inline(always)]
    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_hw_watchpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::HwWatchpointOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_hw_breakpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::HwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl<'a, REGISTERS> gdbstub::target::ext::breakpoints::HwBreakpoint
    for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn add_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        info!("set hw breakpoint {:x} {}", addr, kind);
        let mut id = REGISTERS::hw_breakpoint_start();
        for bp in &self.hw_breakpoints {
            if bp.id == id {
                id += 1;
            }
        }
        info!("Will use ID {}", id);

        if id > REGISTERS::hw_breakpoint_end() {
            info!("Too many HW breakpoints requested");
            return Ok(false);
        }

        self.debug_connection.set_breakpoint(addr as u32, id);
        self.hw_breakpoints.push(SerialDbgHwBreakpoint {
            id: id,
            address: addr as u32,
        });
        Ok(true)
    }

    fn remove_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        info!("rem hw breakpoint {:x} {}", addr, kind);
        let mut id = 0;
        let mut idx = usize::MAX;
        for (i, bp) in self.hw_breakpoints.iter().enumerate() {
            if bp.address == addr as u32 {
                id = bp.id;
                idx = i;
            }
        }

        if idx != usize::MAX {
            info!("Remove ID {}", id);
            self.debug_connection.clear_breakpoint(id);
            self.hw_breakpoints.remove(idx);
            Ok(true)
        } else {
            info!("No breakpoint found to remove");
            Ok(false)
        }
    }
}

impl<'a, REGISTERS> gdbstub::target::ext::breakpoints::SwBreakpoint
    for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn add_sw_breakpoint(&mut self, addr: u32, kind: usize) -> TargetResult<bool, Self> {
        info!("set sw breakpoint {:x} {}", addr, kind);
        let mut duplicate = false;
        for bp in &self.sw_breakpoints {
            if bp.address == addr {
                duplicate = true;
                break;
            }
        }

        if !duplicate {
            let mut old_mem = [0u8; 4];
            self.read_addrs(addr, &mut old_mem).ok();
            let bp = SerialDbgSwBreakpoint {
                address: addr,
                original_code: [old_mem[0], old_mem[1], old_mem[2]],
            };

            let break_opcode = Vec::from(
                &REGISTERS::sw_breakpoint_opcode().1[..REGISTERS::sw_breakpoint_opcode().0],
            );
            self.write_addrs(addr, &break_opcode[..]).ok(); // replace with EBREAK / BREAK
            self.sw_breakpoints.push(bp);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn remove_sw_breakpoint(&mut self, addr: u32, kind: usize) -> TargetResult<bool, Self> {
        info!("rem sw breakpoint {:x} {}", addr, kind);

        let mut original_code = [0u8; 3];
        let mut idx = usize::MAX;
        for (i, bp) in self.sw_breakpoints.iter().enumerate() {
            if bp.address == addr as u32 {
                original_code = bp.original_code;
                idx = i;
            }
        }

        if idx != usize::MAX {
            info!("Restore code {:2x?}", original_code);
            self.write_addrs(addr, &Vec::from(original_code)).ok();
            self.sw_breakpoints.remove(idx);
            Ok(true)
        } else {
            info!("No breakpoint found to remove");
            Ok(false)
        }
    }
}

impl<'a, REGISTERS> gdbstub::target::ext::breakpoints::HwWatchpoint
    for SerialDbgTarget<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    fn add_hw_watchpoint(
        &mut self,
        _addr: u32,
        _len: u32,
        _kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        println!("add hw watchpoint unsupported for now");
        Ok(true)
    }

    fn remove_hw_watchpoint(
        &mut self,
        _addr: u32,
        _len: u32,
        _kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        println!("remove hw watchpoint unsupported for now");
        Ok(true)
    }
}

struct SerialDbgGdbEventLoop<'a, REGISTERS>
where
    REGISTERS: Registers + EspRegisters,
{
    _phantom: &'a PhantomData<REGISTERS>,
}

impl<'a, REGISTERS> run_blocking::BlockingEventLoop for SerialDbgGdbEventLoop<'a, REGISTERS>
where
    REGISTERS: Registers<ProgramCounter = u32> + EspRegisters,
{
    type Target = SerialDbgTarget<'a, REGISTERS>;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;
    type StopReason = SingleThreadStopReason<u32>;

    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<
        run_blocking::Event<Self::StopReason>,
        run_blocking::WaitForStopReasonError<
            <Self::Target as gdbstub::target::Target>::Error,
            <Self::Connection as gdbstub::conn::Connection>::Error,
        >,
    > {
        let res = loop {
            if conn.peek().map(|b| b.is_some()).unwrap_or(true) {
                let byte = conn
                    .read()
                    .map_err(run_blocking::WaitForStopReasonError::Connection)?;

                break Ok(run_blocking::Event::IncomingData(byte));
            }

            match target.debug_connection.pending_message() {
                Some(DeviceMessage::HitBreakpoint(data)) => {
                    info!("breakpoint hit! {:x?}", data);
                    target.registers = data;

                    // restore all sw-breakpoints that
                    // might got temporarly disabled
                    for bp in target.temporarly_disabled_sw_breakpoints.clone() {
                        target.add_sw_breakpoint(bp.address, 0).ok();
                    }
                    target.temporarly_disabled_sw_breakpoints.clear();

                    match target.stepping {
                        true => {
                            // restore any hw-breakpoints
                            let first_hw_breakpoint_id = REGISTERS::hw_breakpoint_start();
                            target
                                .debug_connection
                                .clear_breakpoint(first_hw_breakpoint_id);
                            target
                                .debug_connection
                                .clear_breakpoint(first_hw_breakpoint_id + 1);
                            for i in first_hw_breakpoint_id..(first_hw_breakpoint_id + 1) {
                                for hw_brkpt in &target.hw_breakpoints {
                                    if hw_brkpt.id == i {
                                        target.debug_connection.set_breakpoint(hw_brkpt.address, i);
                                    }
                                }
                            }

                            break Ok(run_blocking::Event::TargetStopped(
                                gdbstub::stub::BaseStopReason::DoneStep,
                            ));
                        }
                        false => {
                            break Ok(run_blocking::Event::TargetStopped(
                                gdbstub::stub::BaseStopReason::HwBreak(()),
                            ));
                        }
                    }
                }
                _ => (),
            }
        };

        res
    }

    fn on_interrupt(
        target: &mut Self::Target,
    ) -> Result<Option<Self::StopReason>, <Self::Target as gdbstub::target::Target>::Error> {
        target.debug_connection.break_execution();
        loop {
            match target.debug_connection.pending_message() {
                Some(DeviceMessage::HitBreakpoint(data)) => {
                    info!("interrupt hit: {:x?}", data);
                    target.registers = data;
                    break;
                }
                _ => (),
            }
        }
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT)))
    }
}
