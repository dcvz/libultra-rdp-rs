use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

use crate::gbi::{GBIResult, GBI, defines::Gfx};
use crate::rdp::RDP;
use crate::rsp::RSP;

const NUM_SEGMENTS: usize = 16;

lazy_static! {
    static ref SEGMENT_TABLE: Arc<Mutex<[usize; NUM_SEGMENTS]>> =
        Arc::new(Mutex::new([0; NUM_SEGMENTS]));
}

pub struct RCP {
    rdp: RDP,
    rsp: RSP,
    gbi: GBI,
}

impl RCP {
    pub fn new() -> RCP {
        RCP {
            rdp: RDP::new(),
            rsp: RSP::new(),
            gbi: GBI::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.gbi.setup();
    }

    /// This funtion is called to process a work buffer.
    /// It takes in a pointer to the start of the work buffer and will
    /// process until it hits a `G_ENDDL` inidicating the end.
    pub fn process_displaylist(&mut self, mut command: *const Gfx) {
        loop {
            // if command returns a new command, we need to run that command
            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, command)
            {
                GBIResult::Recurse(new_command) => {
                    self.process_displaylist(new_command);
                }
                GBIResult::Increase => {
                    command = unsafe { command.add(1) };
                }
                GBIResult::Decrease => {
                    command = unsafe { command.sub(1) };
                }
                GBIResult::SetAddressWithDecrease(new_address) => {
                    let cmd = new_address as *const Gfx;
                    command = unsafe { cmd.sub(1) };
                }
                GBIResult::Return => {
                    return;
                }
                _ => {}
            }

            command = unsafe { command.add(1) }; // We do this since some commands decrement the command pointer
        }
    }

    // MARK: - Helpers

    #[inline(always)]
    pub fn segmented_address(address: usize) -> usize {
        if address & 1 > 0 {
            let segment_index = address >> 24;
            let offset = address & 0x00FFFFFE;

            let segment_address = SEGMENT_TABLE.lock().unwrap()[segment_index as usize];
            if segment_address != 0 {
                return segment_address + offset;
            } else {
                return address;
            }
        } else {
            return address;
        }
    }
}
