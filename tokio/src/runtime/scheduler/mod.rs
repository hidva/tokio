cfg_rt! {
    pub(crate) mod current_thread;
    pub(crate) use current_thread::CurrentThread;

    mod defer;
    use defer::Defer;

    mod inject;
    pub(crate) use inject::Inject;
}

cfg_rt_multi_thread! {
    pub(crate) mod multi_thread;
    pub(crate) use multi_thread::MultiThread;
}

use crate::runtime::driver;

#[derive(Debug, Clone)]
pub(crate) enum Handle {
    #[cfg(feature = "rt")]
    CurrentThread(Arc<current_thread::Handle>),

    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
    MultiThread(Arc<multi_thread::Handle>),

    // TODO: This is to avoid triggering "dead code" warnings many other places
    // in the codebase. Remove this during a later cleanup
    #[cfg(not(feature = "rt"))]
    #[allow(dead_code)]
    Disabled,
}

#[cfg(feature = "rt")]
pub(super) enum Context {
    CurrentThread(current_thread::Context),

    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
    MultiThread(multi_thread::Context),
}

impl Handle {
    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    pub(crate) fn driver(&self) -> &driver::Handle {
        match *self {
            #[cfg(feature = "rt")]
            Handle::CurrentThread(ref h) => &h.driver,

            #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
            Handle::MultiThread(ref h) => &h.driver,

            #[cfg(not(feature = "rt"))]
            Handle::Disabled => unreachable!(),
        }
    }
}

cfg_rt! {
    use crate::future::Future;
    use crate::loom::sync::Arc;
    use crate::runtime::{blocking, task::Id};
    use crate::runtime::context;
    use crate::task::JoinHandle;
    use crate::util::RngSeedGenerator;
    use std::task::Waker;

    impl Handle {
        #[track_caller]
        pub(crate) fn current() -> Handle {
            match context::with_current(Clone::clone) {
                Ok(handle) => handle,
                Err(e) => panic!("{}", e),
            }
        }

        pub(crate) fn blocking_spawner(&self) -> &blocking::Spawner {
            match self {
                Handle::CurrentThread(h) => &h.blocking_spawner,

                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                Handle::MultiThread(h) => &h.blocking_spawner,
            }
        }

        pub(crate) fn spawn<F>(&self, future: F, id: Id) -> JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
        {
            match self {
                Handle::CurrentThread(h) => current_thread::Handle::spawn(h, future, id),

                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                Handle::MultiThread(h) => multi_thread::Handle::spawn(h, future, id),
            }
        }

        pub(crate) fn shutdown(&self) {
            match *self {
                Handle::CurrentThread(_) => {},

                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                Handle::MultiThread(ref h) => h.shutdown(),
            }
        }

        pub(crate) fn seed_generator(&self) -> &RngSeedGenerator {
            match self {
                Handle::CurrentThread(h) => &h.seed_generator,

                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                Handle::MultiThread(h) => &h.seed_generator,
            }
        }

        pub(crate) fn as_current_thread(&self) -> &Arc<current_thread::Handle> {
            match self {
                Handle::CurrentThread(handle) => handle,
                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                _ => panic!("not a CurrentThread handle"),
            }
        }
    }

    cfg_metrics! {
        use crate::runtime::{SchedulerMetrics, WorkerMetrics};

        impl Handle {
            pub(crate) fn num_workers(&self) -> usize {
                match self {
                    Handle::CurrentThread(_) => 1,
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.num_workers(),
                }
            }

            pub(crate) fn num_blocking_threads(&self) -> usize {
                match self {
                    Handle::CurrentThread(handle) => handle.num_blocking_threads(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.num_blocking_threads(),
                }
            }

            pub(crate) fn num_idle_blocking_threads(&self) -> usize {
                match self {
                    Handle::CurrentThread(handle) => handle.num_idle_blocking_threads(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.num_idle_blocking_threads(),
                }
            }

            pub(crate) fn active_tasks_count(&self) -> usize {
                match self {
                    Handle::CurrentThread(handle) => handle.active_tasks_count(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.active_tasks_count(),
                }
            }

            pub(crate) fn scheduler_metrics(&self) -> &SchedulerMetrics {
                match self {
                    Handle::CurrentThread(handle) => handle.scheduler_metrics(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.scheduler_metrics(),
                }
            }

            pub(crate) fn worker_metrics(&self, worker: usize) -> &WorkerMetrics {
                match self {
                    Handle::CurrentThread(handle) => handle.worker_metrics(worker),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.worker_metrics(worker),
                }
            }

            pub(crate) fn injection_queue_depth(&self) -> usize {
                match self {
                    Handle::CurrentThread(handle) => handle.injection_queue_depth(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.injection_queue_depth(),
                }
            }

            pub(crate) fn worker_local_queue_depth(&self, worker: usize) -> usize {
                match self {
                    Handle::CurrentThread(handle) => handle.worker_metrics(worker).queue_depth(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.worker_local_queue_depth(worker),
                }
            }

            pub(crate) fn blocking_queue_depth(&self) -> usize {
                match self {
                    Handle::CurrentThread(handle) => handle.blocking_queue_depth(),
                    #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                    Handle::MultiThread(handle) => handle.blocking_queue_depth(),
                }
            }
        }
    }

    impl Context {
        #[track_caller]
        pub(crate) fn expect_current_thread(&self) -> &current_thread::Context {
            match self {
                Context::CurrentThread(context) => context,
                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                _ => panic!("expected `CurrentThread::Context`")
            }
        }

        pub(crate) fn defer(&self, waker: &Waker) {
            match self {
                Context::CurrentThread(context) => context.defer(waker),
                #[cfg(all(feature = "rt-multi-thread", not(tokio_wasi)))]
                Context::MultiThread(context) => context.defer(waker),
            }
        }

        cfg_rt_multi_thread! {
            #[track_caller]
            pub(crate) fn expect_multi_thread(&self) -> &multi_thread::Context {
                match self {
                    Context::MultiThread(context) => context,
                    _ => panic!("expected `MultiThread::Context`")
                }
            }
        }
    }
}

cfg_not_rt! {
    #[cfg(any(
        feature = "net",
        all(unix, feature = "process"),
        all(unix, feature = "signal"),
        feature = "time",
    ))]
    impl Handle {
        #[track_caller]
        pub(crate) fn current() -> Handle {
            panic!("{}", crate::util::error::CONTEXT_MISSING_ERROR)
        }
    }
}
