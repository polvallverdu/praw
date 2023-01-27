use sndfile::SndFileIO;

extern crate opus;
extern crate praw;
extern crate sndfile;

fn main() {
    let mut encoder =
        opus::Encoder::new(48000, opus::Channels::Stereo, opus::Application::Audio).unwrap();
    encoder
        .set_bitrate(opus::Bitrate::Bits(160 * 1000))
        .unwrap();

    // Convert all to opus
    let mut total_bufs: Vec<Vec<Vec<u8>>> = Vec::new();
    println!("Converting to opus");
    {
        let mut file = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path("./examples/song.flac")
            .unwrap();

        let file_len = file.len().unwrap();
        let mut done = false;
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        while !done {
            total_bufs.push(Vec::new());

            const SECS_PER_PACKET: f32 = 10.0;
            const OPUS_PACKET_LEN: f32 = 100.0 / 1000.0;
            for i in 0..(SECS_PER_PACKET / OPUS_PACKET_LEN) as i32 {
                let mut buf = vec![0.0; (48000.0 * 2.0 * OPUS_PACKET_LEN) as usize];
                let len = file.read_to_slice(buf.as_mut_slice()).unwrap();

                // TODO: LOSING AUDIO?
                //buf.truncate(len*2); // 2 is Channels

                let mut out = vec![0; 512 * 12];
                let len_opus = encoder
                    .encode_float(buf.as_slice(), out.as_mut_slice())
                    .unwrap();
                out.truncate(len_opus);
                if len_opus == 12 {
                    done = true;
                    break;
                }

                //println!("{} : {} : {}", buf.len(), len, len_opus);
                total_bufs.last_mut().unwrap().push(out);
            }
        }
    }

    // Create praw writer
    println!("Writing to praw");
    {
        let mut writer = praw::PrawWriter::new(48000, vec![praw::Channels::Stereo], praw::AudioContainer::Opus);
        for pack in total_bufs {
            writer.add_pack(vec![pack]);
        }

        // Write to file
        writer.save_to_file("./examples/song_opus.praw").unwrap();
    }

    // Read praw file
    println!("Reading and decoding praw");
    {
        let mut reader = praw::PrawFileReader::load("./examples/song_opus.praw").unwrap();
        let mut decoder = opus::Decoder::new(48000, opus::Channels::Stereo).unwrap();

        let mut decoded_file = sndfile::OpenOptions::WriteOnly(sndfile::WriteOptions::new(
            sndfile::MajorFormat::FLAC,
            sndfile::SubtypeFormat::PCM_16,
            sndfile::Endian::File,
            48000,
            2,
        ))
        .from_path("./examples/decoded_opus.flac")
        .unwrap();

        for _ in 0..reader.packs_num {
            let allpack = reader.read_pack();
            let pack = &allpack[0];

            for buf in &pack.data {
                let mut out = vec![0.0; (48000.0 * 2.0 * 0.1) as usize];
                let len = decoder
                    .decode_float(buf.as_slice(), out.as_mut_slice(), false)
                    .unwrap();
                //out.truncate(len*2);

                for i in 0..out.len() {
                    if out[i] > 1.0 {
                        out[i] = 1.0;
                    } else if out[i] < -1.0 {
                        out[i] = -1.0;
                    }
                }

                decoded_file.write_from_slice(out.as_slice()).unwrap();
            }
        }
    }
}
