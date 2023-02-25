use sndfile::SndFileIO;

extern crate opus;
extern crate praw;
extern crate sndfile;

fn open_file(path: &str) -> sndfile::SndFile {
    sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
        .from_path(path)
        .unwrap()
}

fn main() {
    let mut encoder =
        opus::Encoder::new(48000, opus::Channels::Stereo, opus::Application::Audio).unwrap();
    encoder
        .set_bitrate(opus::Bitrate::Bits(160 * 1000))
        .unwrap();

    // Convert all to opus
    let mut total_bufs: Vec<Vec<Vec<Vec<u8>>>> = Vec::new();
    println!("Converting to opus");
    {
        let mut og = open_file("./examples/song.flac");
        let mut drums = open_file("./examples/stems/drums.wav");
        let mut instrument = open_file("./examples/stems/instruments.wav");
        let mut vocals = open_file("./examples/stems/vocals.wav");

        let mut files = [og, drums, instrument, vocals];

        const SECS_PER_PACKET: f32 = 10.0;
        const OPUS_PACKET_LEN: f32 = 100.0 / 1000.0;

        let mut len_opus = 0;
        while len_opus != 12 {
            total_bufs.push(Vec::new());

            for stemi in 0..files.len() {
                // total_bufs[stemi].push(Vec::new());
                total_bufs.last_mut().unwrap().push(Vec::new());
                let packbuf = total_bufs.last_mut().unwrap();
                let mut file = &mut files[stemi];

                for _ in 0..(SECS_PER_PACKET / OPUS_PACKET_LEN) as i32 {
                    let mut buf = vec![0.0; (48000.0 * 2.0 * OPUS_PACKET_LEN) as usize];
                    file.read_to_slice(buf.as_mut_slice()).unwrap();
    
                    let mut out = vec![0; 512 * 12];
                    len_opus = encoder
                        .encode_float(buf.as_slice(), out.as_mut_slice())
                        .unwrap();
                    out.truncate(len_opus);
                    if len_opus == 12 {
                        break;
                    }
    
                    //println!("{} : {} : {}", buf.len(), len, len_opus);
                    packbuf[stemi].push(out);
                }
            }
        }
    }

    // Create praw writer
    println!("Writing to praw");
    {
        let mut writer = praw::PrawWriter::new(
            48000,
            vec![
                praw::Channels::Stereo,
                praw::Channels::Stereo,
                praw::Channels::Stereo,
                praw::Channels::Stereo,
            ],
            praw::AudioContainer::Opus,
        );
        for pack in total_bufs {
            writer.add_pack(pack);
        }

        // Write to file
        writer
            .save_to_file("./examples/song_stems_opus.praw")
            .unwrap();
    }
}
