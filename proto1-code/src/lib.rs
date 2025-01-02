//! Useful routines related to Meridian Comms etc.

#![no_std]

use embassy_time::Duration;

/// A decoder for Meridian Comms
///
pub struct CommsDecoder {
    buffer: u64,
    timings: [u64; 64],
    tlen: usize,
}

/// A Meridial Comms message
#[repr(packed)]
#[derive(defmt::Format)]
pub struct CommsMessage {
    len: u8,
    src: u8,
    dest: u8,
    payload: [u8; 5],
}

impl CommsMessage {
    pub fn new(
        src_type: u8,
        src_addr: u8,
        dest_type: u8,
        dest_addr: u8,
        payload: &[u8],
    ) -> CommsMessage {
        assert!(!payload.is_empty() && payload.len() <= 5);
        let mut ret = Self {
            len: payload.len() as u8,
            src: ((src_type & 0x1F) << 3) | (src_addr & 0x07),
            dest: ((dest_type & 0x1F) << 3) | (dest_addr & 0x07),
            payload: [0; 5],
        };
        ret.payload
            .split_at_mut(payload.len())
            .0
            .copy_from_slice(payload);
        ret
    }

    fn from_buffer(buffer: &[u8]) -> Self {
        let len = (buffer[0] & 0x1F) as usize;
        let mut ret = Self {
            len: buffer[0] & 0x1F,
            src: buffer[1],
            dest: buffer[2],
            payload: [0; 5],
        };
        ret.payload
            .split_at_mut(len)
            .0
            .copy_from_slice(&buffer[3..]);
        ret
    }

    pub fn src_type(&self) -> u8 {
        self.src >> 3
    }

    pub fn src_addr(&self) -> u8 {
        self.src & 0x07
    }

    pub fn dest_type(&self) -> u8 {
        self.dest >> 3
    }

    pub fn dest_addr(&self) -> u8 {
        self.dest & 0x07
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload[0..self.len as usize]
    }
}

const COMMS_ONE_BIT_TIME_THRESHOLD: Duration = Duration::from_micros(750);
const COMMS_ONE_FIVE_BIT_TIME_THRESHOLD: Duration = Duration::from_micros(1250);
const COMMS_TWO_BIT_TIME_THRESHOLD: Duration = Duration::from_micros(1750);
pub const COMMS_TIMEOUT_THRESHOLD: Duration = Duration::from_micros(2250);

impl CommsDecoder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            buffer: 1,
            timings: [0; 64],
            tlen: 0,
        }
    }

    pub fn consume_delta(&mut self, delta: Duration) -> Option<CommsMessage> {
        self.timings[self.tlen] = delta.as_micros();
        self.tlen += 1;
        if delta > COMMS_TIMEOUT_THRESHOLD {
            // We consider this a 'timeout'
            self.buffer <<= 1;
        } else if delta > COMMS_TWO_BIT_TIME_THRESHOLD {
            // Two bit times, always emit 0 then 1
            self.buffer <<= 2;
            self.buffer |= 1;
        } else if delta > COMMS_ONE_FIVE_BIT_TIME_THRESHOLD {
            // One and a half bit times, if last was 1, emit 00 else emit 1
            if self.buffer & 1 == 1 {
                self.buffer <<= 2;
            } else {
                self.buffer <<= 1;
                self.buffer |= 1;
            }
        } else if delta > COMMS_ONE_BIT_TIME_THRESHOLD {
            // One bit, repeat previous value
            let b = self.buffer & 1;
            self.buffer <<= 1;
            self.buffer |= b;
        } else {
            // We assume a glitch and so do nothing to our buffer
        }
        // Next we check to see if we have a full message
        let buffer: [u8; 8] = [
            ((self.buffer >> 56) & 0xFF) as u8,
            ((self.buffer >> 48) & 0xFF) as u8,
            ((self.buffer >> 40) & 0xFF) as u8,
            ((self.buffer >> 32) & 0xFF) as u8,
            ((self.buffer >> 24) & 0xFF) as u8,
            ((self.buffer >> 16) & 0xFF) as u8,
            ((self.buffer >> 8) & 0xFF) as u8,
            (self.buffer & 0xFF) as u8,
        ];
        if buffer[0] == 0xE5 {
            // 5 byte payload packet
            Some(CommsMessage::from_buffer(&buffer[0..]))
        } else if buffer[1] == 0xE4 {
            // 4 byte payload packet
            Some(CommsMessage::from_buffer(&buffer[1..]))
        } else if buffer[2] == 0xE3 {
            // 3 byte payload packet
            Some(CommsMessage::from_buffer(&buffer[2..]))
        } else if buffer[3] == 0xE2 {
            // 2 byte payload packet
            Some(CommsMessage::from_buffer(&buffer[3..]))
        } else if buffer[4] == 0xE1 {
            // 1 byte payload packet
            Some(CommsMessage::from_buffer(&buffer[4..]))
        } else {
            None
        }
    }

    pub fn timings(&self) -> &[u64] {
        &self.timings[0..self.tlen]
    }
}
