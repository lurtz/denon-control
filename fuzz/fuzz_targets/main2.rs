#![no_main]

use std::{
    cell::{Cell, RefCell},
    cmp::min,
    io::Write,
};

use denon_control::{main2, parse_args, ConnectionStream, Logger, ReadStream};
use libfuzzer_sys::fuzz_target;

struct FuzzStream {
    data: RefCell<Vec<u8>>,
    pos: Cell<usize>,
    pos_at_last_peek: Cell<Option<usize>>,
}

impl FuzzStream {
    fn new(data: &[u8]) -> FuzzStream {
        FuzzStream {
            data: RefCell::new(data.to_vec()),
            pos: Cell::new(0),
            pos_at_last_peek: Cell::new(None),
        }
    }
}

impl ReadStream for FuzzStream {
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
                // lets give, the data it needs to end the test
                self.data
                    .replace("PWON\rSICD\rMV555\rMVMAX333\r".as_bytes().to_vec());
                // TODO how to terminate receive thread?
            }
        }
        let length = min(self.data.borrow().len() - cpos, buf.len());
        // println!("length == {}", length);
        // TODO maybe return error if length == 0
        //      returning 0 will terminate the loop
        buf[0..length].copy_from_slice(&self.data.borrow()[cpos..(cpos + length)]);
        self.pos_at_last_peek.replace(Some(cpos));
        Ok(length)
    }

    fn read_exactly(&self, buf: &mut [u8]) -> std::io::Result<()> {
        let cpos = self.pos.get();
        assert!((self.data.borrow().len() - cpos) >= buf.len());
        let _ = self.peekly(buf);
        self.pos.replace(cpos + buf.len());
        Ok(())
    }
}

impl Write for FuzzStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ConnectionStream for FuzzStream {
    fn shutdownly(&self) -> std::io::Result<()> {
        // TODO actually stop object returned by get_readstream()
        self.pos_at_last_peek.replace(Some(self.pos.get()));
        Ok(())
    }

    fn get_readstream(&self) -> std::io::Result<Box<dyn ReadStream>> {
        Ok(Box::new(FuzzStream::new(&self.data.borrow_mut())))
    }
}

struct NoLogger {}

impl Logger for NoLogger {
    fn log(&self, _message: &str) {}
}

fuzz_target!(|data: &[u8]| {
    let fuzz_stream = FuzzStream::new(data);
    let logger = Box::new(NoLogger {});
    let args = parse_args(vec!["blub".to_string(), "--status".to_string()], &*logger);
    let _ = main2(args, Box::new(fuzz_stream), logger);
});
