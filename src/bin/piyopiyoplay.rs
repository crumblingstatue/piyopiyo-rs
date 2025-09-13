use std::{io::Write, process::ExitCode};

fn main() -> ExitCode {
    let path = std::env::args().nth(1).expect("Need .pmd file");
    let data = std::fs::read(&path).unwrap();
    let mut player = piyopiyo::Player::new(&data).unwrap();
    let mut buf = [0; 1024];
    let mut writer = std::io::stdout().lock();
    loop {
        player.render_next(&mut buf);
        let result = writer.write_all(bytemuck::cast_slice(&buf));
        eprint!(
            "Playing {path} {:04}/{:04}\r",
            player.event_cursor,
            player.n_events()
        );
        if let Err(e) = result {
            match e.kind() {
                std::io::ErrorKind::BrokenPipe => return ExitCode::SUCCESS,
                _ => {
                    eprintln!("Error occured while writing to stdout: {e}. Exiting.");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
}
