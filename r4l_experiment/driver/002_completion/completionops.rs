use core::cell::UnsafeCell;
use kernel::bindings;
use kernel::prelude::*;
use kernel::sync::UniqueArc;

pub(crate) struct CompletionFileOps(UnsafeCell<bindings::completion>);

static mut COMPLETION_FILE_OPS: Option<Pin<UniqueArc<CompletionFileOps>>> = None;

impl CompletionFileOps {
    fn new() -> Result<()> {
        pr_info!("CompletionFile(new)\n");

        // SAFETY: 给静态变量初始化。
        unsafe {
            COMPLETION_FILE_OPS = Some(Pin::from(UniqueArc::try_new(CompletionFileOps(
                UnsafeCell::new(bindings::completion::default()),
            ))?));
        }
        Ok(())
    }

    pub(crate) fn init_completion() {
        Self::new().unwrap();
        // SAFETY: 给静态变量初始化赋值，在new之后调用。
        unsafe {
            if let Some(completion) = &COMPLETION_FILE_OPS.as_mut() {
                bindings::init_completion(completion.0.get());
            }
            pr_info!("CompletionFile(init_completion)\n");
        }
    }

    pub(crate) fn wait_for_completion() {
        // SAFETY: 在init_completion之后调用。
        unsafe {
            if let Some(completion) = &COMPLETION_FILE_OPS.as_mut() {
                bindings::wait_for_completion(completion.0.get());
            }
            pr_info!("CompletionFile(wait_for_completion)\n");
        }
    }

    pub(crate) fn complete() {
        // SAFETY: 在init_completion之后调用。
        unsafe {
            if let Some(completion) = &COMPLETION_FILE_OPS.as_mut() {
                bindings::complete(completion.0.get());
            }
            pr_info!("CompletionFile(complete)\n");
        }
    }
}

unsafe impl Send for CompletionFileOps {}
unsafe impl Sync for CompletionFileOps {}

// struct Current {
//     task_struct: bindings::task_struct,
// }

// impl Current {
//     fn new() -> Self {
//         Self {
//             task_struct: unsafe { *Box::from_raw(bindings::get_current()) },
//         }
//     }
// }
