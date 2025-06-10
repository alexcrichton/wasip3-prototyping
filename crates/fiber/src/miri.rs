#![allow(unused_imports, unused_variables, dead_code)]
use crate::stackswitch::*;
use crate::{Result, RunResult, RuntimeFiberStack};
use alloc::boxed::Box;
use alloc::{vec, vec::Vec};
use core::cell::Cell;
use core::ops::Range;
use std::sync::{Arc, Condvar, Mutex};

pub type Error = anyhow::Error;

pub struct FiberStack;

impl FiberStack {
    pub fn new(size: usize, zeroed: bool) -> Result<Self> {
        Ok(FiberStack)
    }

    pub unsafe fn from_raw_parts(base: *mut u8, guard_size: usize, len: usize) -> Result<Self> {
        Ok(FiberStack)
    }

    pub fn is_from_raw_parts(&self) -> bool {
        false
    }

    pub fn from_custom(_custom: Box<dyn RuntimeFiberStack>) -> Result<Self> {
        todo!()
    }

    pub fn top(&self) -> Option<*mut u8> {
        todo!()
    }

    pub fn range(&self) -> Option<Range<usize>> {
        None
    }

    pub fn guard_range(&self) -> Option<Range<*mut u8>> {
        None
    }
}

pub struct Fiber {
    thing: *const u8,
    thread: Option<std::thread::JoinHandle<()>>,
}

pub struct Suspend {
    thing: *const u8,
}

struct State<A, B, C> {
    cond: Condvar,
    state: Mutex<Option<RunResult<A, B, C>>>,
}

unsafe impl<A, B, C> Send for State<A, B, C> {}
unsafe impl<A, B, C> Sync for State<A, B, C> {}

struct ShutUp<T>(T);

unsafe impl<T> Send for ShutUp<T> {}
unsafe impl<T> Sync for ShutUp<T> {}

fn omg<F, A, B, C>(state: Arc<State<A, B, C>>, t: ShutUp<F>)
where
    F: FnOnce(A, &mut super::Suspend<A, B, C>) -> C,
{
    let ShutUp(func) = t;
    // drop(&func);
    let mut lock = state.state.lock().unwrap();

    loop {
        match lock.take() {
            None => {
                lock = state.cond.wait(lock).unwrap();
                continue;
            }
            Some(RunResult::Resuming(thing)) => {
                drop(lock);
                let suspend_thing = Arc::into_raw(state.clone());
                super::Suspend::<A, B, C>::execute(
                    Suspend {
                        thing: suspend_thing.cast(),
                    },
                    thing,
                    func,
                );

                unsafe {
                    drop(Arc::from_raw(suspend_thing));
                }
                break;
            }

            Some(_) => unimplemented!(),
        }
    }
}

impl Fiber {
    pub fn new<F, A, B, C>(stack: &FiberStack, func: F) -> Result<Self>
    where
        F: FnOnce(A, &mut super::Suspend<A, B, C>) -> C,
    {
        let state = Arc::new(State::<A, B, C> {
            cond: Condvar::new(),
            state: Mutex::new(None),
        });

        let thread = unsafe {
            std::thread::Builder::new()
                .spawn_unchecked({
                    let state = state.clone();
                    let func = ShutUp(func);
                    move || {
                        omg(state, func);
                    }
                })
                .unwrap()
        };
        Ok(Fiber {
            thing: Arc::into_raw(state).cast(),
            thread: Some(thread),
        })
    }

    pub(crate) fn resume<A, B, C>(&self, stack: &FiberStack, result: &Cell<RunResult<A, B, C>>) {
        let my_state = self.state();
        let mut lock = my_state.state.lock().unwrap();
        *lock = Some(result.replace(RunResult::Executing));
        my_state.cond.notify_one();
        lock = my_state.cond.wait(lock).unwrap();
        result.set(lock.take().unwrap());
    }

    fn state<A, B, C>(&self) -> &State<A, B, C> {
        unsafe { &*(self.thing as *const State<A, B, C>) }
    }

    pub(crate) fn drop<A, B, C>(&mut self) {
        let state = self.state::<A, B, C>();
        *state.state.lock().unwrap() = Some(RunResult::Exiting);
        state.cond.notify_one();
        self.thread.take().unwrap().join().unwrap();

        unsafe {
            drop(Arc::from_raw(self.thing.cast::<State<A, B, C>>()));
        }
    }
}

impl Suspend {
    pub(crate) fn switch<A, B, C>(&mut self, result: RunResult<A, B, C>) -> A {
        let state = self.state();
        let mut lock = state.state.lock().unwrap();
        assert!(lock.is_none());
        *lock = Some(result);
        state.cond.notify_one();
        lock = state.cond.wait(lock).unwrap();
        match lock.take().unwrap() {
            RunResult::Resuming(a) => a,
            _ => unreachable!(),
        }
    }

    pub(crate) fn exit<A, B, C>(&mut self, result: RunResult<A, B, C>) {
        let state = self.state();
        let mut lock = state.state.lock().unwrap();
        assert!(lock.is_none());
        *lock = Some(result);
        state.cond.notify_one();
        lock = state.cond.wait(lock).unwrap();
        match lock.take().unwrap() {
            RunResult::Exiting => {}
            _ => unreachable!(),
        }
    }

    fn state<A, B, C>(&self) -> &State<A, B, C> {
        unsafe { &*(self.thing as *const State<A, B, C>) }
    }
}
