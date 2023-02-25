// TODO: ESTO ES LO MAS GUARRO QUE HE PROGRAMADO EN TODA MI VIDA. EL DIA QUE ME ABURRA O LOS SERVIDORES PETEN, TOCA REESCRIBIR ESTO :D

use anyhow::Result;
use std::{
    fs::File,
    io::{Read, Seek, Write, SeekFrom},
};

pub enum Channels {
    Mono = 0,
    Stereo = 1,
}
impl Clone for Channels {
    fn clone(&self) -> Self {
        match self {
            Channels::Mono => Channels::Mono,
            Channels::Stereo => Channels::Stereo,
        }
    }
}

pub enum AudioContainer {
    Opus = 0,
    Flac = 1,
}
impl Clone for AudioContainer {
    fn clone(&self) -> Self {
        match self {
            AudioContainer::Opus => AudioContainer::Opus,
            AudioContainer::Flac => AudioContainer::Flac,
        }
    }
}

pub struct Track {
    pub channels: Channels,
}
impl Clone for Track {
    fn clone(&self) -> Self {
        Track {
            channels: self.channels.clone(),
        }
    }
}

pub struct PackedTrack {
    pub track: Track,
    pub data: Vec<Vec<u8>>,
}

pub struct PrawReader {
    header: Vec<u8>,

    pub version: i32,
    pub samplerate: i32,

    pub tracks_num: i32,
    pub tracks: Vec<Track>,
    pub audio_container: AudioContainer,

    pub packs_num: i32,
}

impl PrawReader {
    pub fn load(buf: Vec<u8>) -> Result<PrawReader> {
        let header_copy = buf.clone();
        let mut pointer = 0;

        // Get version as int
        let old_p = pointer;
        pointer += 4;
        let num = &buf[old_p..pointer];
        let version = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);

        // Get samplerate as int
        let old_p = pointer;
        pointer += 4;
        let num = &buf[old_p..pointer];
        let samplerate = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);

        // Get tracks_num as int
        let old_p = pointer;
        pointer += 4;
        let num = &buf[old_p..pointer];
        let tracks_num = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);

        // Get tracks channel (0 = mono, 1 = stereo) as Vec<Track>. Note that each u8 is a track indicating a channel (0 for mono, 1 for stereo)
        let mut tracks = Vec::new();
        for _ in 0..tracks_num {
            let track = buf[pointer];
            pointer += 1;
            let track = Track {
                channels: match track {
                    0 => Channels::Mono,
                    1 => Channels::Stereo,
                    _ => panic!("Invalid channel type"),
                },
            };
            tracks.push(track);
        }

        // Get audio container
        let old_p = pointer;
        pointer += 1;
        let num = &buf[old_p..pointer];
        let audio_container = match num[0] {
            0 => AudioContainer::Opus,
            1 => AudioContainer::Flac,
            _ => panic!("Invalid audio container"),
        };

        // Get packs_num
        let old_p = pointer;
        pointer += 4;
        let num = &buf[old_p..pointer];
        let packs_num = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);

        Ok(PrawReader {
            header: header_copy,
            version,
            samplerate,
            tracks_num,
            tracks,
            audio_container,
            packs_num,
        })
    }

    pub fn read_pack(&mut self, buf: Vec<u8>) -> Vec<PackedTrack> {
        let mut packed_tracks = Vec::new();
        let mut track_packets: Vec<Vec<Vec<u8>>> = vec![Vec::with_capacity(0); self.tracks_num as usize];
        let mut pointer = 0;

        // Get the size of each track
        let mut track_sizes = Vec::new();
        for _ in 0..self.tracks_num {
            let old_p = pointer;
            pointer += 4;
            let num = &buf[old_p..pointer];
            let track_size = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);
            track_sizes.push(track_size);
        }

        // while buf.len() < pointer {
            

        // Get the data for each track
        for i in 0..self.tracks_num as usize {
            let size = track_sizes[i as usize] as usize;
            let totalsize = pointer + size;

            // let old_p = pointer;
            // pointer += size;

            let mut minipacks: Vec<Vec<u8>> = Vec::new();

            while totalsize > pointer  {
                let minipack_size = i32::from_be_bytes([buf[pointer], buf[pointer + 1], buf[pointer + 2], buf[pointer + 3]]) as usize;
                pointer += 4;

                let old_p = pointer;
                pointer += minipack_size;
                let minipack = buf[old_p..pointer].to_vec();
                minipacks.push(minipack);
            }
            track_packets[i] = minipacks;
        }
        // }

        for i in 0..self.tracks_num as usize {
            packed_tracks.push(PackedTrack {
                track: self.tracks[i].clone(),
                data: track_packets[i].clone(),
            });
        }

        packed_tracks
    }

    pub fn get_header(&self) -> Vec<u8> {
        self.header.clone()
    }
}

pub struct PrawWriter {
    pub version: i32,

    pub samplerate: i32,

    pub tracks_num: i32,
    pub tracks: Vec<Track>,
    pub audio_container: AudioContainer,

    packs: Vec<Vec<PackedTrack>>,
}

impl PrawWriter {
    pub fn new(samplerate: i32, tracks: Vec<Channels>, audio_container: AudioContainer) -> PrawWriter {
        PrawWriter {
            version: 1,
            samplerate,
            tracks_num: tracks.len() as i32,
            tracks: tracks
                .into_iter()
                .map(|track| Track { channels: track })
                .collect(),
            audio_container,
            packs: Vec::new(),
        }
    }

    pub fn add_pack(&mut self, pack: Vec<Vec<Vec<u8>>>) {
        let mut packed_tracks = Vec::new();
        
        for i in 0..self.tracks_num {
            packed_tracks.push(PackedTrack {
                track: self.tracks[i as usize].clone(),
                data: pack[i as usize].clone(),
            });
        }
        self.packs.push(packed_tracks);
    }

    pub fn get_header(&self) -> Vec<u8> {
        let mut header = Vec::new();

        // Add version
        header.extend_from_slice(&self.version.to_be_bytes());

        // Add samplerate
        header.extend_from_slice(&self.samplerate.to_be_bytes());

        // Add tracks_num
        header.extend_from_slice(&self.tracks_num.to_be_bytes());

        // Add tracks
        for track in &self.tracks {
            header.push(match track.channels {
                Channels::Mono => 0,
                Channels::Stereo => 1,
            });
        }

        // Add audio_container
        header.push(match self.audio_container {
            AudioContainer::Opus => 0,
            AudioContainer::Flac => 1,
        });

        // Add packs_num
        header.extend_from_slice(&(self.packs.len() as i32).to_be_bytes());

        header
    }

    pub fn get_packs(&self) -> Vec<u8> {
        let mut packs: Vec<u8> = Vec::new();

        // for pack in &self.packs {
        //     let mut pack_data: Vec<u8> = Vec::new();
        //     let opus_pack_len = pack[0].data.len();

        //     for i in 0..opus_pack_len {
        //         // Add the size of each track
        //         for track in pack {
        //             pack_data.extend_from_slice(&(track.data[i].len() as i32).to_be_bytes());
        //         }

        //         // Add the data for each track
        //         for track in pack {
        //             pack_data.extend_from_slice(&track.data[i]);
        //         }
        //     }

        //     packs.extend_from_slice(&(pack_data.len() as i32).to_be_bytes());
        //     packs.extend_from_slice(&pack_data);
        // }

        for pack in &self.packs {
            let mut pack_data: Vec<u8> = Vec::new();

            for track in pack {
                // sum up all the minipacks data of a track
                let mut total_len: i32 = 0;
                for minipack in &track.data {
                    total_len += 4;
                    total_len += minipack.len() as i32;
                }
                // Add the size of each track
                pack_data.extend_from_slice(&total_len.to_be_bytes());
            }

            for track in pack {
                // Add the data for each track
                for minipack in &track.data {
                    pack_data.extend_from_slice(&(minipack.len() as i32).to_be_bytes());
                    pack_data.extend_from_slice(&minipack);
                }
            }

            packs.extend_from_slice(&(pack_data.len() as i32).to_be_bytes());
            packs.extend_from_slice(&pack_data);
        }

        packs
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;

        let header = self.get_header();
        let header_size = (header.len() as i32).to_be_bytes();
        file.write("praw".as_bytes())?;
        file.write(&header_size)?;
        file.write(header.as_slice())?;
        file.write(&self.get_packs())?;

        file.flush()?;

        Ok(())
    }
}

pub struct PrawFileReader {
    /* FOR EASY ACCESS */
    pub version: i32,

    pub samplerate: i32,
    pub tracks_num: i32,
    //pub tracks: Vec<Track>,
    pub packs_num: i32,
    /* FOR EASY ACCESS */
    pub reader: PrawReader,

    file: File,
    past_pack_pos: Vec<u64>,
    pub current_pack: i32,
}

impl PrawFileReader {
    pub fn load(path: &str) -> Result<PrawFileReader> {
        let mut file = File::open(path)?;

        // Get "praw" string (or magic number)
        let mut magic = [0u8; 4];
        file.read(&mut magic)?;
        if magic != "praw".as_bytes() {
            return Err(anyhow::Error::msg("Invalid file"));
        }

        // Get header size as int
        let mut header_size = [0u8; 4];
        file.read(&mut header_size)?;
        let header_size = i32::from_be_bytes(header_size);

        // Get header as Vec<u8>
        let mut header = vec![0u8; header_size as usize];
        file.read(&mut header)?;

        let praw_reader = PrawReader::load(header)?;
        let packs_num = praw_reader.packs_num;

        let mut past_pack_pos = vec![0; packs_num as usize];
        for i in 0..packs_num as usize {
            past_pack_pos[i] = file.stream_position().unwrap() as u64;

            // Get the size of the pack
            let mut pack_size = [0u8; 4];
            file.read(&mut pack_size).unwrap();
            let pack_size = i32::from_be_bytes(pack_size);

            // Skip the pack
            file.seek(SeekFrom::Current(pack_size as i64)).unwrap();
        }
        // Go to the first pack
        file.seek(SeekFrom::Start(past_pack_pos[0])).unwrap();

        Ok(PrawFileReader {
            version: praw_reader.version,
            samplerate: praw_reader.samplerate,
            tracks_num: praw_reader.tracks_num,
            //tracks: praw_reader.tracks,
            packs_num: praw_reader.packs_num,
            reader: praw_reader,
            file,
            past_pack_pos: past_pack_pos,
            current_pack: 0,
        })
    }

    pub fn read_pack(&mut self) -> Result<Vec<PackedTrack>> {
        let buf = self.read_raw_pack()?;

        let pack = self.reader.read_pack(buf);
        self.current_pack += 1;
        Ok(pack)
    }

    pub fn read_raw_pack(&mut self) -> Result<Vec<u8>> {
        if self.current_pack >= self.packs_num {
            return Err(anyhow::Error::msg("No more packs"));
        }
        // Seek to pack
        self.file.seek(SeekFrom::Start(self.past_pack_pos[self.current_pack as usize])).unwrap();

        // Get the size of the pack
        let mut pack_size = [0u8; 4];
        self.file.read(&mut pack_size).unwrap();
        let pack_size = i32::from_be_bytes(pack_size);

        // Read the pack
        let mut buf = vec![0u8; pack_size as usize];
        self.file.read(&mut buf).unwrap();

        return Ok(buf);
    }

    pub fn go_to_pack(&mut self, pack: i32) -> Result<()> {
        if pack < 0 || pack > self.packs_num {
            return Err(anyhow::Error::msg("Invalid pack number"));
        }

        let pack_pos = self.past_pack_pos[pack as usize];
        match self.file.seek(SeekFrom::Start(pack_pos)) {
            Ok(_) => {
                self.current_pack = pack;
                return Ok(());
            },
            Err(_) => {
                return Err(anyhow::Error::msg("Failed to seek to pack"));
            },
        };
    }

    pub fn go_to_second(&mut self, second: i32) -> Result<()> {
        if second < 0 || second >= self.reader.packs_num*10 {
            panic!("Second out of range");
        }

        let pack = (second as f32/10.0).floor() as i32;
        self.go_to_pack(pack)
    }
}
