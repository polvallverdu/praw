use sndfile::SndFileIO;

extern crate opus;
extern crate ropus;
extern crate sndfile;

fn main() {
    let mut encoder =
        opus::Encoder::new(48000, opus::Channels::Stereo, opus::Application::Audio).unwrap();
    encoder
        .set_bitrate(opus::Bitrate::Bits(128 * 1000))
        .unwrap();

    // Convert all to opus
    let mut total_bufs: Vec<Vec<Vec<u8>>> = Vec::new();
    println!("Converting to opus");
    {
        let mut file = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path("./examples/song.flac")
            .unwrap();
          
        let file_len = file.len().unwrap();
        let mut counter = 0;
        file.seek(std::io::SeekFrom::Start(0));

        while counter < file_len {
            total_bufs.push(Vec::new());

            // 10secs / 0.02secsperpacket = 500 packets
            for i in 0..500 {
                let mut buf = vec![0.0; (48000.0 * 2.0 * 0.12) as usize];
                let len = file.read_to_slice(buf.as_mut_slice()).unwrap();

                counter+=(len as u64);

                // TODO: LOSING AUDIO?
                //buf.truncate(len);

                let mut out = vec![0; 512 * 5];
                let len = encoder
                    .encode_float(buf.as_slice(), out.as_mut_slice())
                    .unwrap();
                out.truncate(len);

                total_bufs.last_mut().unwrap().push(out);
            }
        }
    }

    // Create ropus writer
    println!("Writing to ropus");
    {
        let mut writer = ropus::RopusWriter::new(48000, vec![ropus::Channels::Stereo]);
        for pack in total_bufs {
            writer.add_pack(vec![pack]);
        }

        // Write to file
        writer.save_to_file("./examples/song.ropus").unwrap();
    }

    // Read ropus file
    println!("Reading and decoding ropus");
    {
        let mut reader = ropus::RopusFileReader::load("./examples/song.ropus").unwrap();
        let mut decoder = opus::Decoder::new(48000, opus::Channels::Stereo).unwrap();

        let mut decoded_file = sndfile::OpenOptions::WriteOnly(sndfile::WriteOptions::new(
            sndfile::MajorFormat::FLAC,
            sndfile::SubtypeFormat::PCM_16,
            sndfile::Endian::File,
            48000,
            2,
        ))
        .from_path("./examples/decoded.flac")
        .unwrap();

        for _ in 0..reader.packs_num {
            let allpack = reader.read_pack();
            let pack = &allpack[0];

            for buf in &pack.data {
              let mut out = vec![0.0; (48000.0 * 2.0 * 0.12) as usize];
              let len = decoder
                  .decode_float(buf.as_slice(), out.as_mut_slice(), false)
                  .unwrap();
              out.truncate(len);
  
              decoded_file.write_from_slice(out.as_slice()).unwrap();
            }
        }
    }
}
