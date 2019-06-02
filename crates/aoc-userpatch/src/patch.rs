#![allow(clippy::unreadable_literal)]
use crate::InstallOptions;
use std::{fmt, str};

#[derive(Clone)]
pub struct Feature {
    pub name: &'static str,
    pub optional: bool,
    pub affects_sync: bool,
    enabled: bool,
    patches: &'static [Injection],
}

impl fmt::Debug for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Feature {{ \"{}\", optional: {:?}, affects_sync: {:?}, enabled: {:?} }}",
            self.name, self.optional, self.affects_sync, self.enabled
        )
    }
}

impl Feature {
    fn assert_optional(&self) {
        assert!(
            self.optional,
            "cannot toggle non-optional feature \"{}\"",
            self.name
        );
    }

    pub fn enable(&mut self, enabled: bool) {
        self.assert_optional();
        self.enabled = enabled;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Describes a patch as an offset and a hexadecimal string.
struct Injection(u32, &'static str);

/// Decode a hexadecimal string to a list of byte values.
fn decode_hex(hexa: &str) -> Vec<u8> {
    assert_eq!(
        hexa.len() % 2,
        0,
        "hex string must have length divisible by 2"
    );
    let mut bytes = Vec::with_capacity(hexa.len() / 2);
    for c in hexa.as_bytes().chunks(2) {
        let high = char::from(c[0])
            .to_digit(16)
            .expect("expected only hexadecimal characters");
        let low = char::from(c[1])
            .to_digit(16)
            .expect("expected only hexadecimal characters");
        bytes.push((high * 16 + low) as u8);
    }
    bytes
}

/// Overwrite bytes in buffer at an offset.
fn apply_patch(buffer: &mut [u8], offset: usize, patch: &[u8]) {
    let end = offset + patch.len();
    (&mut buffer[offset..end]).copy_from_slice(&patch);
}

include!(concat!(env!("OUT_DIR"), "/injections.rs"));

pub fn get_available_features() -> &'static [Feature] {
    &FEATURES
}

fn configure_features(options: InstallOptions) -> Vec<Feature> {
    FEATURES
        .iter()
        .cloned()
        .map(|mut f| {
            f.enable(match f.name {
                "Widescreen interface style" => options.widescreen_command_bar,
                "Windowed mode support" => options.windowed_mode,
                "Port forwarding support" => options.upnp,
                "Darken mini-map red" => options.alternate_red,
                "Darken mini-map purple" => options.alternate_purple,
                "Darken mini-map grey" => options.alternate_gray,
                "Population caps to 1000" => options.extend_population_caps,
                "Snow/ice terrain removal" => options.replace_snow_with_grass,
                "Enable water animation" => options.water_animation,
                "Precision scrolling system" => options.precision_scrolling,
                "Shift group appending" => options.shift_group_append,
                "Keydown object hotkeys" => options.keydown_hotkeys,
                "New save filename format" => options.savegame_format,
                "Multiple building queue" => options.multiple_queue,
                "Original patrol default" => options.original_patrol_delay,
                "Disable water movement" => !options.water_movement,
                "Disable weather system" => !options.weather_system,
                "Disable custom terrains" => !options.custom_terrains,
                "Disable terrain underwater" => !options.terrain_underwater,
                "Numeric age display" => options.numeric_age_display,
                _ => f.enabled(),
            });
            f
        })
        .collect()
}

/// Install UserPatch 1.5 into a buffer containing a 1.0c executable.
pub fn install_into(exe_buffer: &[u8], options: InstallOptions) -> Vec<u8> {
    let features = configure_features(options);

    let mut bigger_buffer = exe_buffer.to_vec();
    bigger_buffer.extend(&vec![0; (3072 * 1024) - exe_buffer.len()]);

    for feature in features.iter() {
        if !feature.enabled() {
            continue;
        }

        let Feature { patches, .. } = feature;
        for Injection(addr, patch) in patches.iter() {
            let patch = decode_hex(&patch);
            let mut addr = *addr as usize;
            while addr > bigger_buffer.len() {
                eprintln!(
                    "WARNING decreasing addr {:x} {:x}",
                    addr,
                    addr - bigger_buffer.len()
                );
                addr -= bigger_buffer.len()
            }
            apply_patch(&mut bigger_buffer, addr, &patch);
        }
    }
    bigger_buffer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstallOptions;
    use std::fs::{read, write};

    #[test]
    fn decode_hex_test() {
        assert_eq!(decode_hex("ABCDEF"), vec![0xAB_u8, 0xCD_u8, 0xEF_u8]);
        assert_eq!(decode_hex("123456"), vec![0x12_u8, 0x34_u8, 0x56_u8]);
    }

    #[test]
    fn apply_patch_test() {
        let mut buffer = vec![0u8; 256];
        apply_patch(&mut buffer, 8, &[1u8; 8]);
        assert_eq!(
            &buffer[0..24],
            &[
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            ]
        );
        apply_patch(&mut buffer, 10, &[2u8; 4]);
        assert_eq!(
            &buffer[0..24],
            &[
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 1u8, 2u8, 2u8, 2u8, 2u8, 1u8, 1u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            ]
        );
    }

    #[test]
    fn produce_bare_up15() {
        use std::{env, path::PathBuf};
        if let Ok(base) = env::var("AOCDIR") {
            let base = PathBuf::from(base);
            let aoc = read(base.join("Age2_x1/age2_x1.0c.exe")).unwrap();
            let up15 = install_into(&aoc, InstallOptions::bare());
            write(base.join("Age2_x1/age2_x1.rs.exe"), &up15).unwrap();
        }
    }

    #[test]
    fn get_patch_options_test() {
        eprintln!("{:#?}", get_available_features());
    }
}
