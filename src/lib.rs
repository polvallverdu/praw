// TODO: ESTO ES LO MAS GUARRO QUE HE PROGRAMADO EN TODA MI VIDA. EL DIA QUE ME ABURRA O LOS SERVIDORES PETEN, TOCA REESCRIBIR ESTO :D

use anyhow::Result;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
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

pub struct RopusReader {
    pub version: i32,

    pub samplerate: i32,
    pub tracks_num: i32,
    pub tracks: Vec<Track>,

    pub packs_num: i32,
    pack_seek: Vec<i32>,

    current_pack: i32,
}

impl RopusReader {
    pub fn load(buf: Vec<u8>) -> Result<RopusReader> {
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
            let mut track = buf[0];
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

        // Get packs_num
        let old_p = pointer;
        pointer += 4;
        let num = &buf[old_p..pointer];
        let packs_num = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);

        // Get pack_seek
        let mut pack_seek = Vec::new();
        for _ in 0..packs_num {
            let old_p = pointer;
            pointer += 4;
            let pack = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);
            pack_seek.push(pack);
        }

        Ok(RopusReader {
            version,
            samplerate,
            tracks_num,
            tracks,
            packs_num,
            pack_seek,
            current_pack: 0,
        })
    }

    pub fn read_pack(&mut self, buf: Vec<u8>) -> Vec<PackedTrack> {
        let mut packed_tracks = Vec::new();
        let mut track_packets: Vec<Vec<Vec<u8>>> = vec![Vec::new(); self.tracks_num as usize];
        let mut pointer = 0;

        while buf.len() > 0 {
            // Get the size of each track
            let mut track_sizes = Vec::new();
            for _ in 0..self.tracks_num {
                let old_p = pointer;
                pointer += 4;
                let num = &buf[old_p..pointer];
                let track_size = i32::from_be_bytes([num[0], num[1], num[2], num[3]]);
                track_sizes.push(track_size);
            }

            // Get the data for each track
            for i in 0..self.tracks_num as usize {
                let size = track_sizes[i as usize] as usize;

                let old_p = pointer;
                pointer += size;

                let mut data = buf[old_p..pointer].to_vec();
                track_packets[i].push(data);
            }
        }

        for i in 0..self.tracks_num as usize {
            packed_tracks.push(PackedTrack {
                track: self.tracks[i].clone(),
                data: track_packets[i].clone(),
            });
        }

        self.current_pack += 1;

        packed_tracks
    }
}

pub struct RopusWriter {
    pub version: i32,

    pub samplerate: i32,
    pub tracks_num: i32,
    pub tracks: Vec<Track>,

    packs: Vec<Vec<PackedTrack>>,
}

impl RopusWriter {
    pub fn new(samplerate: i32, tracks: Vec<Channels>) -> RopusWriter {
        RopusWriter {
            version: 1,
            samplerate,
            tracks_num: tracks.len() as i32,
            tracks: tracks
                .into_iter()
                .map(|track| Track { channels: track })
                .collect(),
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

        // Add packs_num
        header.extend_from_slice(&(self.packs.len() as i32).to_be_bytes());

        // Add pack_seek
        // TODO: CHANGE
        let mut pack_seek = 0;
        for _ in 0..self.packs.len() {
            header.extend_from_slice(&(pack_seek as i32).to_be_bytes());
            pack_seek += 4
                + (4 * self.tracks_num as usize)
                + self.packs[0]
                    .iter()
                    .map(|track| track.data.len())
                    .sum::<usize>();
        }

        header
    }

    pub fn get_packs(&self) -> Vec<u8> {
        let mut packs = Vec::new();

        for pack in &self.packs {
            let opus_pack_len = pack[0].data.len();

            for i in 0..opus_pack_len {
                // Add the size of each track
                for track in pack {
                    packs.extend_from_slice(&(track.data.len() as i32).to_be_bytes());
                }

                // Add the data for each track
                for track in pack {
                    packs.extend_from_slice(&track.data[i]);
                }
            }
        }

        packs
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;

        let header = self.get_header();
        let header_size = (header.len() as i32).to_be_bytes();
        let mut len = file.write(&header_size)?;
        println!("Header size: {}", len);
        len = file.write(header.as_slice())?;
        println!("Header: {:?}", len);
        len = file.write(&self.get_packs())?;
        println!("Packs: {:?}", len);

        file.flush()?;

        Ok(())
    }
}

pub struct RopusFileReader {
    /* FOR EASY ACCESS */
    pub version: i32,

    pub samplerate: i32,
    pub tracks_num: i32,
    //pub tracks: Vec<Track>,
    pub packs_num: i32,
    /* FOR EASY ACCESS */
    pub reader: RopusReader,

    file: File,
}

impl RopusFileReader {
    pub fn load(path: &str) -> Result<RopusFileReader> {
        let mut file = File::open(path)?;

        // Get header size as int
        let mut header_size = [0u8; 4];
        file.read(&mut header_size)?;
        println!("Header size: {:?}", header_size);
        let header_size = i32::from_be_bytes(header_size);
        println!("Header size: {}", header_size);

        // Get header as Vec<u8>
        let mut header = vec![0u8; header_size as usize];
        file.read(&mut header)?;

        let ropus_reader = RopusReader::load(header)?;

        Ok(RopusFileReader {
            version: ropus_reader.version,
            samplerate: ropus_reader.samplerate,
            tracks_num: ropus_reader.tracks_num,
            //tracks: ropus_reader.tracks,
            packs_num: ropus_reader.packs_num,
            reader: ropus_reader,
            file,
        })
        // Get version as int
        // let mut version = [0u8; 4];
        // file.read(&mut version)?;
        // let version = i32::from_be_bytes(version);

        // // Get samplerate as int
        // let mut samplerate = [0u8; 4];
        // file.read(&mut samplerate)?;
        // let samplerate = i32::from_be_bytes(samplerate);

        // // Get tracks_num as int
        // let mut tracks_num = [0u8; 4];
        // file.read(&mut tracks_num)?;
        // let tracks_num = i32::from_be_bytes(tracks_num);

        // // Get tracks channel (0 = mono, 1 = stereo) as Vec<Track>. Note that each u8 is a track indicating a channel (0 for mono, 1 for stereo)
        // let mut tracks = Vec::new();
        // for _ in 0..tracks_num {
        //     let mut track = [0u8; 1];
        //     file.read(&mut track)?;
        //     let track = Track {
        //         channels: match track[0] {
        //             0 => Channels::Mono,
        //             1 => Channels::Stereo,
        //             _ => panic!("Invalid channel type"),
        //         },
        //     };
        //     tracks.push(track);
        // }

        // // Get packs_num
        // let mut packs_num = [0u8; 4];
        // file.read(&mut packs_num)?;
        // let packs_num = i32::from_be_bytes(packs_num);

        // // Get pack_seek
        // let mut pack_seek = Vec::new();
        // for _ in 0..packs_num {
        //     let mut pack = [0u8; 4];
        //     file.read(&mut pack)?;
        //     let pack = i32::from_be_bytes(pack);
        //     pack_seek.push(pack);
        // }

        // Ok(RopusFileReader {
        //     version,
        //     samplerate,
        //     tracks_num,
        //     tracks,
        //     packs_num,
        //     pack_seek,
        //     file,
        //     current_pack: 0,
        // })
    }

    pub fn read_pack(&mut self) -> Vec<PackedTrack> {
        // Seek to the current pack
        let pack_pos = self.reader.pack_seek[self.reader.current_pack as usize] as u64;
        self.file.seek(SeekFrom::Start(pack_pos)).unwrap();

        // Get the size of the pack
        let mut pack_size = [0u8; 4];
        self.file.read(&mut pack_size).unwrap();
        let pack_size = i32::from_be_bytes(pack_size);

        // Read the pack
        let mut buf = vec![0u8; pack_size as usize];
        self.file.read(&mut buf).unwrap();

        self.reader.read_pack(buf)
    }
}

// pub struct RopusFileWriter
