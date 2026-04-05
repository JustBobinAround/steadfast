use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

/// Shared wake flag
struct WakeFlag {
    woke: AtomicBool,
}

type WakerData = Arc<WakeFlag>;

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_waker);

unsafe fn clone(data: *const ()) -> RawWaker {
    let arc = unsafe { WakerData::from_raw(data as *const WakeFlag) };
    let cloned = arc.clone();
    std::mem::forget(arc);
    RawWaker::new(Arc::into_raw(cloned) as *const (), &VTABLE)
}

unsafe fn wake(data: *const ()) {
    let arc = unsafe { WakerData::from_raw(data as *const WakeFlag) };
    arc.woke.store(true, Ordering::Release);
}

unsafe fn wake_by_ref(data: *const ()) {
    let arc = unsafe { &*(data as *const WakeFlag) };
    arc.woke.store(true, Ordering::Release);
}

unsafe fn drop_waker(data: *const ()) {
    drop(unsafe { WakerData::from_raw(data as *const WakeFlag) });
}

/// Minimal executor
pub fn run<F: Future>(future: F) -> F::Output {
    let wake_flag = Arc::new(WakeFlag {
        woke: AtomicBool::new(true), // start "woken"
    });

    let raw_waker = RawWaker::new(Arc::into_raw(wake_flag.clone()) as *const (), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut cx = Context::from_waker(&waker);

    let mut future = Box::pin(future);

    loop {
        if wake_flag.woke.swap(false, Ordering::Acquire) {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(val) => return val,
                Poll::Pending => {}
            }
        }
    }
}
