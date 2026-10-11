// MPEG-PS (Program Stream) demuxer.
// Ported to Rust from PL_MPEG by Dominic Szablewski (MIT licensed).

use super::buffer::Buffer;

const START_PACK: i32 = 0xBA;
const START_SYSTEM: i32 = 0xBB;

pub const PACKET_PRIVATE: i32 = 0xBD;
pub const PACKET_AUDIO_1: i32 = 0xC0;
pub const PACKET_AUDIO_4: i32 = 0xC3;
pub const PACKET_VIDEO_1: i32 = 0xE0;

pub const INVALID_TS: f64 = -1.0;

pub struct Packet {
    pub ty: i32,
    pub pts: f64,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct DemuxedStreams {
    pub video: Vec<u8>,
    pub audio: Vec<u8>,
    pub num_video_streams: i32,
    pub num_audio_streams: i32,
}

pub struct Demux {
    buffer: Buffer,
    start_code: i32,
    has_pack_header: bool,
    has_system_header: bool,
    has_headers: bool,
    num_audio_streams: i32,
    num_video_streams: i32,
    current_length: usize,
    next_type: i32,
    next_length: usize,
    next_pts: f64,
}

impl Demux {
    pub fn new(data: Vec<u8>) -> Self {
        let mut s = Self {
            buffer: Buffer::new(data),
            start_code: -1,
            has_pack_header: false,
            has_system_header: false,
            has_headers: false,
            num_audio_streams: 0,
            num_video_streams: 0,
            current_length: 0,
            next_type: -1,
            next_length: 0,
            next_pts: INVALID_TS,
        };
        s.has_headers();
        s
    }

    pub fn append(&mut self, data: &[u8]) {
        self.buffer.append(data);
    }

    pub fn num_video_streams(&self) -> i32 {
        self.num_video_streams
    }

    pub fn num_audio_streams(&self) -> i32 {
        self.num_audio_streams
    }

    pub fn has_headers(&mut self) -> bool {
        if self.has_headers {
            return true;
        }

        // Pack header
        if !self.has_pack_header {
            if self.start_code != START_PACK && self.buffer.find_start_code(START_PACK) == -1 {
                return false;
            }
            self.start_code = START_PACK;
            if !self.buffer.has(64) {
                return false;
            }
            self.start_code = -1;
            if self.buffer.read(4) != 0x02 {
                return false;
            }
            self.decode_time(); // system clock reference
            self.buffer.skip(1);
            self.buffer.skip(22); // mux_rate * 50
            self.buffer.skip(1);
            self.has_pack_header = true;
        }

        // System header
        if !self.has_system_header {
            if self.start_code != START_SYSTEM && self.buffer.find_start_code(START_SYSTEM) == -1 {
                return false;
            }
            self.start_code = START_SYSTEM;
            if !self.buffer.has(56) {
                return false;
            }
            self.start_code = -1;
            self.buffer.skip(16); // header_length
            self.buffer.skip(24); // rate bound
            self.num_audio_streams = self.buffer.read(6) as i32;
            self.buffer.skip(5); // misc flags
            self.num_video_streams = self.buffer.read(5) as i32;
            self.has_system_header = true;
        }

        self.has_headers = true;
        true
    }

    fn decode_time(&mut self) -> f64 {
        let mut clock = (self.buffer.read(3) as i64) << 30;
        self.buffer.skip(1);
        clock |= (self.buffer.read(15) as i64) << 15;
        self.buffer.skip(1);
        clock |= self.buffer.read(15) as i64;
        self.buffer.skip(1);
        clock as f64 / 90000.0
    }

    pub fn decode(&mut self) -> Option<Packet> {
        if !self.has_headers() {
            return None;
        }

        if self.current_length > 0 {
            let bits = self.current_length << 3;
            if !self.buffer.has(bits) {
                return None;
            }
            self.buffer.skip(bits);
            self.current_length = 0;
            self.buffer.discard_read_bytes();
        }

        if self.start_code != -1 {
            let code = self.start_code;
            return self.decode_packet(code);
        }

        loop {
            self.start_code = self.buffer.next_start_code();
            let sc = self.start_code;
            if sc == PACKET_VIDEO_1
                || sc == PACKET_PRIVATE
                || (PACKET_AUDIO_1..=PACKET_AUDIO_4).contains(&sc)
            {
                return self.decode_packet(sc);
            }
            if sc == -1 {
                return None;
            }
        }
    }

    fn decode_packet(&mut self, ty: i32) -> Option<Packet> {
        if !self.buffer.has(16 << 3) {
            return None;
        }
        self.start_code = -1;

        self.next_type = ty;
        let mut length = self.buffer.read(16) as i64;
        length -= self.buffer.skip_bytes(0xff) as i64; // stuffing

        if self.buffer.read(2) == 0x01 {
            self.buffer.skip(16);
            length -= 2;
        }

        let pts_dts_marker = self.buffer.read(2);
        if pts_dts_marker == 0x03 {
            self.next_pts = self.decode_time();
            self.buffer.skip(40); // skip DTS
            length -= 10;
        } else if pts_dts_marker == 0x02 {
            self.next_pts = self.decode_time();
            length -= 5;
        } else if pts_dts_marker == 0x00 {
            self.next_pts = INVALID_TS;
            self.buffer.skip(4);
            length -= 1;
        } else {
            return None;
        }

        if length < 0 {
            return None;
        }
        self.next_length = length as usize;
        self.get_packet()
    }

    fn get_packet(&mut self) -> Option<Packet> {
        let bits = self.next_length << 3;
        if !self.buffer.has(bits) {
            return None;
        }
        let data = self.buffer.peek_bytes(self.next_length)?.to_vec();
        let packet = Packet {
            ty: self.next_type,
            pts: self.next_pts,
            data,
        };
        self.current_length = self.next_length;
        self.next_length = 0;
        Some(packet)
    }
}

pub fn demux_all(data: Vec<u8>) -> DemuxedStreams {
    let mut demux = Demux::new(data);
    let mut out = DemuxedStreams {
        num_video_streams: demux.num_video_streams(),
        num_audio_streams: demux.num_audio_streams(),
        ..Default::default()
    };

    while let Some(packet) = demux.decode() {
        if packet.ty == PACKET_VIDEO_1 {
            out.video.extend_from_slice(&packet.data);
        } else if packet.ty == PACKET_AUDIO_1 {
            out.audio.extend_from_slice(&packet.data);
        }
    }

    out
}
