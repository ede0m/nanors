use crate::encoding;
use std::error::Error;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::time::SystemTime;

const POW_LOCAL_WORKERS: u64 = 6;
pub const RECV_DIFFICULTY: &str = "fffffe0000000000";
pub const DEFAULT_DIFFICULTY: &str = "fffffff800000000";

//https://docs.nano.org/integration-guides/work-generation/#work-calculation-details
pub fn pow_local(previous: [u8; 32], threshold: &[u8; 8]) -> Result<[u8; 8], Box<dyn Error>> {
    let threshold = threshold.clone();
    let (tx, rx): (Sender<[u8; 8]>, Receiver<[u8; 8]>) = mpsc::channel();
    let found = Arc::new(Mutex::new(false));
    //let now = SystemTime::now();
    let mut handles = vec![];
    // dispatch workers
    for i in 0..POW_LOCAL_WORKERS {
        let (sender, arc) = (tx.clone(), found.clone());
        let handle = std::thread::spawn(move || {
            pow_local_segment(i, &previous, &threshold, sender, arc);
        });
        handles.push(handle);
    }
    let mut work = rx.recv().unwrap(); // recv will block.
                                       /*
                                       let elapsed_min = (now.elapsed()?.as_secs()) as f64 / 60.0;
                                       println!(
                                           "pow complete in {} minutes. work: {:02x?} -> {:02x?}",
                                           elapsed_min,
                                           work,
                                           encoding::nano_work_hash(&previous, &work)
                                       );
                                       */
    *found.lock().unwrap() = true;
    for handle in handles {
        handle.join().unwrap();
    }
    work.reverse(); // work hex string seems to be LE
    Ok(work)
}

fn pow_local_segment(
    i: u64,
    previous: &[u8; 32],
    threshold: &[u8; 8],
    sender: Sender<[u8; 8]>,
    found: Arc<Mutex<bool>>,
) {
    let seg_size = 0xffffffffffffffff / POW_LOCAL_WORKERS;
    let (low, high) = (seg_size * i, seg_size * (i + 1));
    for nonce in low..high {
        let nonce = nonce.to_le_bytes();
        if let Ok(output) = encoding::nano_work_hash(previous, &nonce) {
            // blake2b output in le
            if u64::from_le_bytes(output) >= u64::from_be_bytes(*threshold) {
                sender.send(nonce).unwrap();
                break;
            }
        }
        if *found.lock().unwrap() {
            break;
        }
    }
}
