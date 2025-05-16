use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;
use serde::{Serialize, Deserialize};
use tungstenite::connect;
use ureq;
use hostname;
use sysinfo::{System, SystemExt};
use winapi::um::processthreadsapi::GetSystemTimes;
use winapi::shared::minwindef::FILETIME;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Stats {
    pub node_id: String,
    pub cpu: f32,
    pub mem: f32,
    pub funds: f32,
}


fn get_cpu_usage() -> f32 {
    unsafe {
        let mut idle_time = FILETIME::default();
        let mut kernel_time = FILETIME::default();
        let mut user_time = FILETIME::default();

        if GetSystemTimes(&mut idle_time, &mut kernel_time, &mut user_time) != 0 {
            let idle1 = filetime_to_u64(&idle_time);
            let kernel1 = filetime_to_u64(&kernel_time);
            let user1 = filetime_to_u64(&user_time);

            std::thread::sleep(std::time::Duration::from_secs(1));

            let mut idle_time2 = FILETIME::default();
            let mut kernel_time2 = FILETIME::default();
            let mut user_time2 = FILETIME::default();

            GetSystemTimes(&mut idle_time2, &mut kernel_time2, &mut user_time2);

            let idle2 = filetime_to_u64(&idle_time2);
            let kernel2 = filetime_to_u64(&kernel_time2);
            let user2 = filetime_to_u64(&user_time2);

            let idle_diff = idle2 - idle1;
            let total_diff = (kernel2 - kernel1) + (user2 - user1);

            if total_diff == 0 {
                0.0
            } else {
                100.0 - (idle_diff as f64 * 100.0 / total_diff as f64) as f32
            }
        } else {
            0.0
        }
    }
}

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}


pub fn spawn_stats_sender(stats: Arc<Mutex<Stats>>) {
    thread::spawn({
        let stats_clone = Arc::clone(&stats);
        move || {
            let url = Url::parse("ws://127.0.0.1:9001").unwrap();

            if let Ok((mut socket, _)) = connect(url) {
                let mut sys = System::new_all();

                loop {
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    std::thread::sleep(Duration::from_secs(1));

                    let cpu = get_cpu_usage();
                    let mem = sys.used_memory() as f32 / sys.total_memory() as f32 * 100.0;
                    let node_id = hostname::get().unwrap().to_string_lossy().to_string();

                    // 🚀 Only lock when writing!
                    {
                        let mut stats_lock = stats_clone.lock().unwrap();
                        stats_lock.cpu = cpu;
                        stats_lock.mem = mem;
                        stats_lock.node_id = node_id.clone();
                    }

                    let json = {
                        let stats_snapshot = stats_clone.lock().unwrap().clone();
                        serde_json::to_string(&stats_snapshot).unwrap()
                    };

                    println!("✅ Sending stats: {}", json);
                    let _ = socket.send(tungstenite::Message::Text(json.clone()));
                    let _ = ureq::post("http://127.0.0.1:8000/nodes/update")
                        .set("Content-Type", "application/json")
                        .send_string(&json);

                    std::thread::sleep(Duration::from_secs(45));
                }
            } else {
                eprintln!("❌ Failed to connect to WebSocket server.");
            }
        }
    });
}
