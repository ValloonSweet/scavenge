cfg_if! {
    if #[cfg(unix)] {
        use std::process::Command;

        pub fn get_device_id(path: &str) -> String {
            let output = Command::new("stat")
                .arg(path)
                .arg("-c %D")
                .output()
                .expect("failed to execute 'stat -c %D'");
            String::from_utf8(output.stdout).expect("not utf8")
        }

        // On unix, get the device id from 'df' command
        fn get_device_id_unix(path: &str) -> String {
            let output = Command::new("df")
                 .arg(path)
                 .arg("--output=source")
                 .output()
                 .expect("failed to execute 'df --output=source'");
             let source = String::from_utf8(output.stdout).expect("not utf8");
             source.split('\n').collect::<Vec<&str>>()[1].to_string()
         }

        // On macos, use df and 'diskutil info <device>' to get the Device Block Size line and extract the size
        fn get_sector_size_macos(path: &str) -> u64 {
            let source = get_device_id_unix(path);
            let output = Command::new("diskutil")
                .arg("info")
                .arg(source)
                .output()
                .expect("failed to execute 'diskutil info'");
            let source = String::from_utf8(output.stdout).expect("not utf8");
            let mut sector_size: u64 = 0;
            for line in source.split('\n').collect::<Vec<&str>>() {
                if line.trim().starts_with("Device Block Size") {
                    let source = line.rsplit(' ').collect::<Vec<&str>>()[1];  // e.g. in reverse: "Bytes 512 Size Block Device"
                    sector_size = source.parse::<u64>().unwrap();
                }
            }
            if sector_size == 0 {
                panic!("Abort: Unable to determine disk physical sector size from diskutil info")
            }
            sector_size
        }

        // On unix, use df and lsblk to extract the device sector size
        fn get_sector_size_unix(path: &str) -> u64 {
            let source = get_device_id_unix(path);
            let output = Command::new("lsblk")
                .arg(source)
                .arg("-o")
                .arg("PHY-SeC")
                .output()
                .expect("failed to execute 'lsblk -o PHY-SeC'");

            let sector_size = String::from_utf8(output.stdout).expect("not utf8");
            let sector_size = sector_size.split('\n').collect::<Vec<&str>>()[1].trim();

            sector_size.parse::<u64>().unwrap()
        }

        pub fn get_sector_size(path: &str) -> u64 {
            if cfg!(target_os = "macos") {
                get_sector_size_macos(path)
            } else {
                get_sector_size_unix(path)
            }
        }

    } else {
        extern crate winapi;
        use std::os::windows::ffi::OsStrExt;
        use std::ffi::OsStr;
        use std::iter::once;
        use std::ffi::CString;
        use std::path::Path;

        pub fn get_device_id(path: &String) -> String {
            let path_encoded: Vec<u16> = OsStr::new(path).encode_wide().chain(once(0)).collect();
            let mut volume_encoded: Vec<u16> = OsStr::new(path).encode_wide().chain(once(0)).collect();
            if unsafe {
                winapi::um::fileapi::GetVolumePathNameW(
                    path_encoded.as_ptr(),
                    volume_encoded.as_mut_ptr(),
                    path.chars().count() as u32
                )
            } == 0  {
                panic!("get volume path name");
            };
            let res = String::from_utf16_lossy(&volume_encoded);
            let v: Vec<&str> = res.split('\u{00}').collect();
            String::from(v[0])
        }

        pub fn get_sector_size(path: &String) -> u64 {
            let path_encoded = Path::new(path);
            let parent_path = path_encoded.parent().unwrap().to_str().unwrap();
            let parent_path_encoded = CString::new(parent_path).unwrap();
            let mut sectors_per_cluster  = 0u32;
            let mut bytes_per_sector  = 0u32;
            let mut number_of_free_cluster  = 0u32;
            let mut total_number_of_cluster  = 0u32;
            if unsafe {
                winapi::um::fileapi::GetDiskFreeSpaceA(
                    parent_path_encoded.as_ptr(),
                    &mut sectors_per_cluster,
                    &mut bytes_per_sector,
                    &mut number_of_free_cluster,
                    &mut total_number_of_cluster
                )
            } == 0  {
                panic!("get sector size, filename={}",path);
            };
            bytes_per_sector as u64
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;

    #[test]
    fn test_get_device_id() {
        if cfg!(unix) {
            assert_ne!("", get_device_id(&"Cargo.toml".to_string()));
        }
    }

    #[test]
    fn test_get_sector_size() {
        // this should be true for any platform where this test runs
        // but it doesn't exercise all platform variants
        let cwd = env::current_dir().unwrap();
        let test_string = cwd.into_os_string().into_string().unwrap();
        info!("{}", test_string);
        assert_ne!(0, get_sector_size(&test_string));
    }
}
