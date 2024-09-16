#![no_main]

use std::{
    cell::Cell,
    cmp::min,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use denon_control::{thread_func_impl, ReadStream};
use libfuzzer_sys::fuzz_target;

struct FuzzStream<'a> {
    data: &'a [u8],
    pos: Cell<usize>,
    pos_at_last_peek: Cell<Option<usize>>,
}

impl<'a> FuzzStream<'a> {
    fn new(data: &'a [u8]) -> FuzzStream<'a> {
        FuzzStream {
            data,
            pos: Cell::new(0),
            pos_at_last_peek: Cell::new(None),
        }
    }
}

impl<'a> ReadStream for FuzzStream<'a> {
    fn peekly(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        let cpos = self.pos.get();
        // println!(
        //     "buf.len() == {}, cpos == {}, data.len() == {}, data == {:?}",
        //     buf.len(),
        //     cpos,
        //     self.data.len(),
        //     self.data
        // );
        // this check is at first iteration always true, needs more adjustment
        if let Some(old_pos) = self.pos_at_last_peek.get() {
            if old_pos == cpos {
                // implementation did not extract any data anymore. Test is done
                return Ok(0);
            }
        }
        let length = min(self.data.len() - cpos, buf.len());
        // println!("length == {}", length);
        // TODO maybe return error if length == 0
        //      returning 0 will terminate the loop
        buf[0..length].copy_from_slice(&self.data[cpos..(cpos + length)]);
        self.pos_at_last_peek.replace(Some(cpos));
        Ok(length)
    }

    fn read_exactly(&self, buf: &mut [u8]) -> std::io::Result<()> {
        let cpos = self.pos.get();
        assert!((self.data.len() - cpos) >= buf.len());
        let _ = self.peekly(buf);
        self.pos.replace(cpos + buf.len());
        Ok(())
    }
}

fuzz_target!(|data: &[u8]| {
    let fuzz_stream = FuzzStream::new(data);
    let state = Arc::new(Mutex::new(HashMap::new()));
    let _ = thread_func_impl(&fuzz_stream, state);
});
