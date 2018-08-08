use chan;
use futures::sync::mpsc;
use futures::{Future, Sink};
use reader::ReadReply;
use std::mem;
use std::sync::{Arc, Mutex};
use std::u64;

use libc::{c_void, uint64_t};

extern "C" {
    pub fn find_best_deadline_avx2(
        scoops: *mut c_void,
        nonce_count: uint64_t,
        gensig: *const c_void,
        best_deadline: *mut uint64_t,
        best_offset: *mut uint64_t,
    ) -> ();

    pub fn find_best_deadline_avx(
        scoops: *mut c_void,
        nonce_count: uint64_t,
        gensig: *const c_void,
        best_deadline: *mut uint64_t,
        best_offset: *mut uint64_t,
    ) -> ();

    pub fn find_best_deadline_sse2(
        scoops: *mut c_void,
        nonce_count: uint64_t,
        gensig: *const c_void,
        best_deadline: *mut uint64_t,
        best_offset: *mut uint64_t,
    ) -> ();
}

pub struct NonceData {
    pub height: u64,
    pub deadline: u64,
    pub nonce: u64,
    pub reader_task_processed: bool,
}

pub fn create_worker_task(
    rx_read_replies: chan::Receiver<ReadReply>,
    tx_empty_buffers: chan::Sender<Arc<Mutex<Vec<u8>>>>,
    tx_nonce_data: mpsc::Sender<NonceData>,
) -> impl FnOnce() {
    move || {
        for read_reply in rx_read_replies {
            let buffer = read_reply.buffer;
            if read_reply.len == 0 {
                tx_empty_buffers.send(buffer.clone());
                continue;
            }

            let mut bs = buffer.lock().unwrap();

            let mut deadline: u64 = u64::MAX;
            let mut offset: u64 = 0;
            let padded = pad(&mut bs, read_reply.len, 8 * 64);
            unsafe {
                if is_x86_feature_detected!("avx2") {
                    find_best_deadline_avx2(
                        mem::transmute(bs.as_ptr()),
                        (read_reply.len as u64 + padded as u64) / 64,
                        mem::transmute(read_reply.gensig.as_ptr()),
                        &mut deadline,
                        &mut offset,
                    );
                } else if is_x86_feature_detected!("avx") {
                    find_best_deadline_avx(
                        mem::transmute(bs.as_ptr()),
                        (read_reply.len as u64 + padded as u64) / 64,
                        mem::transmute(read_reply.gensig.as_ptr()),
                        &mut deadline,
                        &mut offset,
                    );
                } else {
                    find_best_deadline_sse2(
                        mem::transmute(bs.as_ptr()),
                        (read_reply.len as u64 + padded as u64) / 64,
                        mem::transmute(read_reply.gensig.as_ptr()),
                        &mut deadline,
                        &mut offset,
                    );
                }
            }

            tx_nonce_data
                .clone()
                .send(NonceData {
                    height: read_reply.height,
                    deadline: deadline,
                    nonce: offset + read_reply.start_nonce,
                    reader_task_processed: read_reply.finished,
                }).wait()
                .expect("failed to send nonce data");
            tx_empty_buffers.send(buffer.clone());
        }
    }
}

fn pad(b: &mut [u8], l: usize, p: usize) -> usize {
    let r = p - l % p;
    if r != p {
        for i in 0..r {
            b[i] = b[0];
        }
        r
    } else {
        0
    }
}
