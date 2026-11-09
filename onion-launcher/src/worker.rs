//! One background thread for the slow shell work (the installed-app scan and
//! icon loading), so the window never waits on Windows. Urgent jobs (icons the
//! screen needs right now) jump the queue. Results come back through
//! `take()` after a WM_LOADED message.

use crate::config::App;
use crate::shell::{self, Pixels};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

pub enum Job {
    Icon(String, u32),
    Installed,
}

pub enum Done {
    Icon(String, u32, Option<Pixels>),
    Installed(Vec<App>),
}

struct Shared {
    jobs: Mutex<VecDeque<Job>>,
    wake: Condvar,
    done: Mutex<Vec<Done>>,
    posted: AtomicBool,
    hwnd: isize,
    msg: u32,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

pub fn start(hwnd: HWND, msg: u32) {
    let sh = Shared { jobs: Mutex::default(), wake: Condvar::new(), done: Mutex::default(), posted: AtomicBool::new(false), hwnd: hwnd.0 as isize, msg };
    if SHARED.set(sh).is_err() {
        return;
    }
    std::thread::spawn(|| {
        shell::com_init();
        let sh = SHARED.get().unwrap();
        // an icon can be asked for twice (once queued, once again when it is on screen)
        let mut seen = std::collections::HashSet::new();
        loop {
            let job = {
                let mut q = sh.jobs.lock().unwrap();
                loop {
                    if let Some(j) = q.pop_front() {
                        break j;
                    }
                    q = sh.wake.wait(q).unwrap();
                }
            };
            let done = match job {
                Job::Icon(src, px) => {
                    if !seen.insert((src.clone(), px)) {
                        continue;
                    }
                    let p = shell::load_icon(&src, px as i32);
                    Done::Icon(src, px, p)
                }
                Job::Installed => Done::Installed(shell::installed_apps()),
            };
            sh.done.lock().unwrap().push(done);
            // one wake-up message per batch, not one per icon
            if !sh.posted.swap(true, Ordering::AcqRel) {
                unsafe {
                    let _ = PostMessageW(HWND(sh.hwnd as _), sh.msg, WPARAM(0), LPARAM(0));
                }
            }
        }
    });
}

pub fn push(job: Job, urgent: bool) {
    let Some(sh) = SHARED.get() else { return };
    let mut q = sh.jobs.lock().unwrap();
    if urgent { q.push_front(job) } else { q.push_back(job) }
    sh.wake.notify_one();
}

pub fn take() -> Vec<Done> {
    let Some(sh) = SHARED.get() else { return vec![] };
    sh.posted.store(false, Ordering::Release);
    std::mem::take(&mut *sh.done.lock().unwrap())
}
