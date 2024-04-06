// SPDX-License-Identifier: GPL-2.0
//! Rust conpletion sample.

use core::result::Result::Err;

use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::task::Task;
use kernel::{chrdev, file};

mod completionops;
use completionops::CompletionFileOps;

module! {
  type: Completion,
  name: "completion",
  author: "Tester",
  description: "Example of Kernel's completion mechanism",
  license: "GPL",
}

const GLOBALMEM_SIZE: usize = 0x1000;

static GLOBALMEM_BUF: Mutex<[u8; GLOBALMEM_SIZE]> = unsafe { Mutex::new([0u8; GLOBALMEM_SIZE]) };

struct CompletionFile {
    data: &'static Mutex<[u8; GLOBALMEM_SIZE]>,
}

#[vtable]
impl file::Operations for CompletionFile {
    type Data = Box<Self>;

    fn open(_shared: &(), _file: &file::File) -> Result<Box<Self>> {
        pr_info!("CompletionFile(open)\n");
        Ok(Box::try_new(CompletionFile {
            data: &GLOBALMEM_BUF,
        })?)
    }

    fn write(
        this: &Self,
        _file: &file::File,
        reader: &mut impl kernel::io_buffer::IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("CompletionFile -pid={:?}--(write)\n", Task::current().pid(),);
        let mut globalmem = this.data.lock();
        let len = reader.len();
        globalmem[0] = len as u8;
        reader.read_slice(&mut globalmem[1..=len])?;
        CompletionFileOps::complete();
        Ok(len)
    }

    fn read(
        this: &Self,
        _file: &file::File,
        writer: &mut impl kernel::io_buffer::IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        if writer.is_empty() || offset > 0 {
            return Ok(0);
        }
        pr_info!("CompletionFile -pid={:?}--(read)\n", Task::current().pid(),);
        CompletionFileOps::wait_for_completion();
        let globalmem = this.data.lock();
        let len = globalmem[0] as usize;
        writer.write_slice(&globalmem[1..(len + 1)])?;

        Ok(len as usize)
    }
}

struct Completion {
    _dev: Pin<Box<chrdev::Registration<2>>>,
}

impl kernel::Module for Completion {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Completion driver demo (init)");
        // CompletionFileOps::new()?;
        CompletionFileOps::init_completion();
        let mut completion_reg = chrdev::Registration::new_pinned(name, 0, module)?;
        completion_reg.as_mut().register::<CompletionFile>()?;
        completion_reg.as_mut().register::<CompletionFile>()?;

        Ok(Completion {
            _dev: completion_reg,
        })
    }
}

impl Drop for Completion {
    fn drop(&mut self) {
        pr_info!("Completion module is being unloaded now (exit)\n");
    }
}
