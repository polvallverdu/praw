use sndfile::SndFileIO;

extern crate opus;
extern crate praw;
extern crate sndfile;

fn main() {
    // Read praw file
    println!("Reading and decoding praw");
    {
        let mut reader = praw::PrawFileReader::load("./examples/song_opus.praw").unwrap();
        let mut decoder = opus::Decoder::new(48000, opus::Channels::Stereo).unwrap();

        let mut decoded_file = sndfile::OpenOptions::WriteOnly(sndfile::WriteOptions::new(
            sndfile::MajorFormat::WAV,
            sndfile::SubtypeFormat::PCM_32,
            sndfile::Endian::File,
            48000,
            2,
        ))
        .from_path("./examples/decoded_opus.wav")
        .unwrap();

        for _ in reader.current_pack..reader.packs_num {
            let allpack = reader.read_pack().unwrap();
            let pack = &allpack[0];

            for buf in &pack.data {
                let mut out = vec![0.0; (48000.0 * 2.0 * 0.1) as usize];
                decoder
                    .decode_float(buf.as_slice(), out.as_mut_slice(), false)
                    .unwrap();
                //out.truncate(len*2);

                const THRESHOLD: f32 = 0.9999999;
                for i in 0..out.len() {
                    if out[i] >= THRESHOLD {
                        out[i] = THRESHOLD;
                    } else if out[i] <= -THRESHOLD {
                        out[i] = -THRESHOLD;
                    }
                }

                decoded_file.write_from_slice(out.as_slice()).unwrap();
            }
        }
    }
}
