use crate::multimedia::{
    input::{InputHandler, KeyboardEvent, MouseEvent},
    MultimediaError, Result,
};

pub struct WasmInput {
    grabbed: bool,
    relative_mode: bool,
}

impl WasmInput {
    pub fn new() -> Result<Self> {
        Ok(Self {
            grabbed: false,
            relative_mode: false,
        })
    }
}

impl InputHandler for WasmInput {
    fn handle_keyboard(&mut self, _event: KeyboardEvent) -> Result<()> {
        // TODO: Forward to SPICE protocol
        Ok(())
    }

    fn handle_mouse(&mut self, _event: MouseEvent) -> Result<()> {
        // TODO: Forward to SPICE protocol
        Ok(())
    }

    fn grab_input(&mut self, grab: bool) -> Result<()> {
        self.grabbed = grab;
        // TODO: Use Pointer Lock API
        Ok(())
    }

    fn is_grabbed(&self) -> bool {
        self.grabbed
    }

    fn set_relative_mouse(&mut self, relative: bool) -> Result<()> {
        self.relative_mode = relative;
        // TODO: Configure pointer lock for relative mode
        Ok(())
    }

    fn warp_mouse(&mut self, _x: i32, _y: i32) -> Result<()> {
        // Not supported in browsers
        Ok(())
    }
}