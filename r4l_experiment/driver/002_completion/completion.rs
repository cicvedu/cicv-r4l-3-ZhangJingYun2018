// SPDX-License-Identifier: GPL-2.0
//! Rust conpletion sample.

use core::result::Result::Err;

use core::cell::UnsafeCell;
use kernel::bindings;
use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::{chrdev, file};

module! {
  type: Completion,
  name: "completion",
  author: "Tester",
  description: "Example of Kernel's completion mechanism",
  license: "GPL",
}

const GLOBALMEM_SIZE: usize = 0x1000;

static GLOBALMEM_BUF: Mutex<[u8; GLOBALMEM_SIZE]> = unsafe { Mutex::new([0u8; GLOBALMEM_SIZE]) };

static mut COMPLETION_FILE_OPS: CompletionFileOps = CompletionFileOps(None);

struct CompletionFileOps(Option<UnsafeCell<bindings::completion>>);

impl CompletionFileOps {
    pub(crate) fn new() {
        pr_info!("CompletionFile(new)\n");
        unsafe {
            COMPLETION_FILE_OPS.0 = Some(UnsafeCell::new(bindings::completion::default()));
        }
        pr_info!("CompletionFile(new){:?}\n", COMPLETION_FILE_OPS.0);
    }

    pub(crate) fn init_completion() {
        unsafe {
            if let Some(completion) = COMPLETION_FILE_OPS.0.as_mut() {
                bindings::init_completion(completion.get());
            }
            pr_info!("CompletionFile(init_completion)\n");
        }
    }

    pub(crate) fn wait_for_completion() {
        unsafe {
            if let Some(completion) = COMPLETION_FILE_OPS.0.as_mut() {
                bindings::wait_for_completion(completion.get());
            }
            pr_info!("CompletionFile(wait_for_completion)\n");
        }
    }

    pub(crate) fn complete() {
        unsafe {
            if let Some(completion) = COMPLETION_FILE_OPS.0.as_mut() {
                bindings::complete(completion.get());
            }
            pr_info!("CompletionFile(complete)\n");
        }
    }
}

unsafe impl Send for CompletionFileOps {}
unsafe impl Sync for CompletionFileOps {}

struct CompletionFile {
    data: &'static Mutex<[u8; GLOBALMEM_SIZE]>,
}

struct Current {
    task_struct: bindings::task_struct,
}

impl Current {
    fn new() -> Self {
        Self {
            task_struct: unsafe { *Box::from_raw(bindings::get_current()) },
        }
    }
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
        let current = Current::new();
        let name = unsafe { CStr::from_char_ptr(current.task_struct.comm.as_ptr()) };
        pr_info!(
            "CompletionFile -pid={:?}-pname={:?}-(write)\n",
            current.task_struct.pid,
            name
        );
        let mut globalmem = this.data.lock();
        pr_info!("CompletionFile(write-1)\n");
        let len = reader.len();
        globalmem[0] = len as u8;
        reader.read_slice(&mut globalmem[1..=len])?;
        CompletionFileOps::complete();
        pr_info!("CompletionFile(write-2)\n");
        // pr_info!("write_len{}\n",len);
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
        let current = Current::new();
        let name = unsafe { CStr::from_char_ptr(current.task_struct.comm.as_ptr()) };
        pr_info!(
            "CompletionFile -pid={:?}-pname={:?}-(read)\n",
            current.task_struct.pid,
            name
        );
        CompletionFileOps::wait_for_completion();
        pr_info!("CompletionFile(read-1)\n");
        let globalmem = this.data.lock();
        pr_info!("CompletionFile(read-2)\n");
        let len = globalmem[0] as usize;
        writer.write_slice(&globalmem[1..(len + 1)])?;
        // pr_info!("read_len{}\n",len);
        // pr_info!("read_len{}\n",_writer.len());
        Ok(len as usize)
    }
}

struct Completion {
    _dev: Pin<Box<chrdev::Registration<1>>>,
}

impl kernel::Module for Completion {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Completion driver demo (init)");
        CompletionFileOps::new();
        CompletionFileOps::init_completion();
        let mut completion_reg = chrdev::Registration::new_pinned(name, 0, module)?;
        completion_reg.as_mut().register::<CompletionFile>()?;
        Ok(Completion {
            _dev: completion_reg,
        })
    }
}

impl Drop for Completion {
    fn drop(&mut self) {
        pr_info!("Completion module is being unloaded now (exit)\n");
        unsafe {
            COMPLETION_FILE_OPS.0 = None;
        }
    }
}
