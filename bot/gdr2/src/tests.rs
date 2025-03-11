#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_empty_replay() {
        let replay = Replay::new();
        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();
        assert_eq!(imported.framerate, 240.0);
        assert_eq!(imported.author, "");
        assert_eq!(imported.description, "");
        assert_eq!(imported.inputs.len(), 0);
        assert_eq!(imported.deaths.len(), 0);
    }

    #[test]
    fn test_basic_metadata() {
        let mut replay = Replay::new();
        replay.author = "zeozeozeo".to_string();
        replay.description = "Test replay".to_string();
        replay.duration = 12.5;
        replay.game_version = 22074;
        replay.framerate = 360.0;
        replay.seed = 42;
        replay.coins = 3;
        replay.ldm = true;
        replay.platformer = true;

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert_eq!(imported.author, "zeozeozeo");
        assert_eq!(imported.description, "Test replay");
        assert_eq!(imported.duration, 12.5);
        assert_eq!(imported.game_version, 22074);
        assert_eq!(imported.framerate, 360.0);
        assert_eq!(imported.seed, 42);
        assert_eq!(imported.coins, 3);
        assert_eq!(imported.ldm, true);
        assert_eq!(imported.platformer, true);
    }

    #[test]
    fn test_bot_info() {
        let mut replay = Replay::new();
        replay.bot_info = Bot {
            name: "ReplayBot".to_string(),
            version: 2,
        };

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert_eq!(imported.bot_info.name, "ReplayBot");
        assert_eq!(imported.bot_info.version, 2);
    }

    #[test]
    fn test_level_info() {
        let mut replay = Replay::new();
        replay.level_info = Level {
            id: 128,
            name: "Stereo Madness".to_string(),
        };

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert_eq!(imported.level_info.id, 128);
        assert_eq!(imported.level_info.name, "Stereo Madness");
    }

    #[test]
    fn test_inputs() {
        let mut replay = Replay::new();
        replay.platformer = true; // Enable platformer mode to test different buttons

        // Add some player 1 inputs
        replay.inputs.push(Input::new(60, 1, false, true)); // Jump press at frame 60
        replay.inputs.push(Input::new(90, 1, false, false)); // Jump release at frame 90

        // Add some player 2 inputs
        replay.inputs.push(Input::new(75, 2, true, true)); // Left press at frame 75
        replay.inputs.push(Input::new(120, 2, true, false)); // Left release at frame 120

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert_eq!(imported.inputs.len(), 4);

        // Check if inputs are sorted by frame
        assert_eq!(imported.inputs[0].frame, 60);
        assert_eq!(imported.inputs[1].frame, 75);
        assert_eq!(imported.inputs[2].frame, 90);
        assert_eq!(imported.inputs[3].frame, 120);

        // Check player 1 inputs
        let p1_inputs: Vec<_> = imported.inputs.iter().filter(|i| !i.player2).collect();
        assert_eq!(p1_inputs.len(), 2);
        assert_eq!(p1_inputs[0].button, 1);
        assert_eq!(p1_inputs[0].down, true);
        assert_eq!(p1_inputs[1].button, 1);
        assert_eq!(p1_inputs[1].down, false);

        // Check player 2 inputs
        let p2_inputs: Vec<_> = imported.inputs.iter().filter(|i| i.player2).collect();
        assert_eq!(p2_inputs.len(), 2);
        assert_eq!(p2_inputs[0].button, 2);
        assert_eq!(p2_inputs[0].down, true);
        assert_eq!(p2_inputs[1].button, 2);
        assert_eq!(p2_inputs[1].down, false);
    }

    // Add a new test for non-platformer mode
    #[test]
    fn test_inputs_non_platformer() {
        let mut replay = Replay::new();
        replay.platformer = false; // Disable platformer mode

        // Add inputs with different buttons (they should all become button 1 when imported)
        replay.inputs.push(Input::new(60, 2, false, true)); // Will become jump
        replay.inputs.push(Input::new(90, 3, true, true)); // Will become jump

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert_eq!(imported.inputs.len(), 2);
        assert_eq!(imported.inputs[0].button, 1); // Should be jump
        assert_eq!(imported.inputs[1].button, 1); // Should be jump
    }

    #[test]
    fn test_deaths() {
        let mut replay = Replay::new();
        replay.deaths = vec![100, 250, 500, 750];

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert_eq!(imported.deaths, vec![100, 250, 500, 750]);
    }

    #[test]
    fn test_invalid_version() {
        let replay = Replay::new();
        let mut data = replay.export_data().unwrap();
        data[3] = 99; // Set version to 99

        assert!(matches!(
            Replay::import_data(&data).unwrap_err(),
            Error::UnsupportedVersion(99)
        ));
    }

    #[test]
    fn test_varint_encoding() {
        let mut writer = BinaryWriter::new();

        // Test various numbers
        let test_numbers = vec![0, 1, 127, 128, 16383, 16384, 2097151, 2097152];

        for &num in &test_numbers {
            writer.write_varint(num);
        }

        let binding = writer.into_vec();
        let mut reader = BinaryReader::new(&binding);

        for &expected in &test_numbers {
            assert_eq!(reader.read_varint().unwrap(), expected);
        }
    }

    #[test]
    fn test_string_encoding() {
        let test_strings = vec![
            "",
            "Hello",
            "Test 123",
            "Special chars: !@#$%^&*()",
            "Unicode: 🎮🎲🎯",
        ];

        let mut writer = BinaryWriter::new();
        for s in &test_strings {
            writer.write_string(s);
        }

        let binding = writer.into_vec();
        let mut reader = BinaryReader::new(&binding);
        for expected in &test_strings {
            assert_eq!(reader.read_string().unwrap(), *expected);
        }
    }

    #[test]
    fn test_file_io() {
        let mut replay = Replay::new();
        replay.author = "zeozeozeo".to_string();
        replay.description = "File I/O test".to_string();
        replay.inputs.push(Input::new(30, 1, false, true));
        replay.deaths.push(100);

        // Write to temporary file
        let temp_path = std::env::temp_dir().join("test_replay.gdr");
        replay.export_to_file(&temp_path).unwrap();

        // Read back and verify
        let imported = Replay::import_from_file(&temp_path).unwrap();
        assert_eq!(imported.author, "zeozeozeo");
        assert_eq!(imported.description, "File I/O test");
        assert_eq!(imported.inputs.len(), 1);
        assert_eq!(imported.deaths.len(), 1);

        // Clean up
        std::fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_platformer_mode() {
        let mut replay = Replay::new();
        replay.platformer = true;

        // Add some platformer-specific inputs
        replay.inputs.push(Input::new(30, 2, false, true)); // Left
        replay.inputs.push(Input::new(60, 3, false, true)); // Right
        replay.inputs.push(Input::new(90, 1, false, true)); // Jump

        let data = replay.export_data().unwrap();
        let imported = Replay::import_data(&data).unwrap();

        assert!(imported.platformer);
        assert_eq!(imported.inputs.len(), 3);
        assert_eq!(imported.inputs[0].button, 2); // Left
        assert_eq!(imported.inputs[1].button, 3); // Right
        assert_eq!(imported.inputs[2].button, 1); // Jump
    }

    #[test]
    fn test_load() {
        let replay = Replay::import_data(include_bytes!("../data/Aeternus.gdr2")).unwrap();

        assert_eq!(replay.author, "Andarian");
        assert_eq!(replay.framerate, 360.0);
        assert_eq!(replay.inputs.len(), 643);
    }
}
