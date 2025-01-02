# Meridian Comms - Electrical interface

The electrical interface for Meridian Comms is a single-wire 5 volt signalled bus with a common ground reference.
The connection is an electrical _bus_ meaning that hubs are entirely passive.

The signalling is a modified manchester-style encoding where each bit of data is guaranteed to have at least
one transition. Because the bus is zero (ground) when idle, there is a small amount of timing-related pain which
has to be dealt with in the case of a zero at the end of the sequence.

The signalling rate is 1kHz. Decoding the signal is easily done by means of counting bit-times between rising edges.

Start with a 1 in the 'last value' register

1. If the difference from one rising edge to the next is a whole bit-time, repeat the `last value`.
2. If the difference is 2x bit times, emit `01` and set `last value` to 1.
3. If the difference is 1.5x bit times, if last was 1, emit `00`, else emit `1` and invert `last value`.
4. If it has been more than 2x bit times since the last rising edge, regardless, emit a zero if the last packet was not complete.

Messages are sent in groups of 8 bits, with packet lengths varying, though the first byte seems to indicate packet length
in some fashion. In reality, simply using timing to decide if the packet is done seems easiest. i.e. if we reach say 4x bit time
with no new rising edge, we've definitely finished receiving a packet, and we can then process it.

## Connectors

Meridian Comms is provided on three kinds of interface, either the DIN plugs, BNC connectors, or RJ45 (speakerlink).

The DIN5 interface has two different kinds of plugs - speaker interlinks which are DIN5-180 and components which
have DIN5-240. <https://www.meridianunplugged.com/wiki/uploads/5serleads.pdf> provides a good set of examples of these.

In BNC, the braid is signal ground, the core is the signal.

In Speakerlink, pin 1 is comms signal, pin 2 is comms ground.

## Message shape

All messages are multiples of eight bits. Messages are sent MSB first, byte by byte.

The first byte of the message is a preamble of 3 set bits, and then a message
length value (1 through 5) `111s ssss`

The second byte of the message appears to be the source of the message.

The third byte of the message seems to be the destination of the message.

We are making the assumption that all the source/destination values are a combination
of the comms type and address. The manuals for the 561, 562, g68, etc. suggest
there are eight address values possible, whereas types in those manuals are 1, 2,
and 3.

Bytes four, through eight, if present, are payload bytes. As far as we can tell,
there is _always_ a fourth byte, sometimes a fifth byte, and we've never seen
bytes six, seven, or eight, in the wild yet.

Thus we think the types of devices are one of:

- CD Player (1) (G68 manual)
- FM? Tuner (2) (G68 manual)
- DVD Player (3) (G68 manual)
- Surround? Controller (???) (TBD by observation)

Given that, we can construct some assumptions about the format of the source/destination
fields. It looks, from messages seen on a bus, that the bottom three bits are the device
address and the rest is the comms type; though there may be more in there we're unaware of
as yet.

## Transmitting a message

If we assume a struct along the lines of:

```rust
struct CommsMessage {
    src_type: u8,
    src_addr: u8,
    dst_type: u8,
    dst_addr: u8,
    payload: CommsPayload,
}

enum CommsPayload {
    One(u8),
    Two(u8, u8),
    //...
}
```

Then we construct the outgoing message as follows:

1. First, we assemble a full message length by orring in the top
   the three one-bits (preamble) to the payload length and we
   can call this pre_len
2. Then we assemble the source and destination bytes
3. Then we add on the payload bytes
4. We take that byte sequence [pre_len,src,dst,payload1,...] and we
   hand off to the wire encoder

The encoder operates as follows:

1. Compute the number of half-bit times we need to send the message
   `(pre_len & 0x1f)` is the message length in bytes, excluding the
   three header bytes, so we do:
   - len = len + 3
   - len = len \* 16
   - len = len + 1
     At the end of that, we have the full length of the message
     in bytes (ie 4 or 5) multiplied by 16, plus one.
2. We queue up the full message, including the preamble bits, and
   enable transmission, ensuring that comms pin is low
3. Transmission runs on a 500µS timer, every 500µS we perform:
   1. If sending the first half of a bit, if sending zero, raise pin,
      If sending one, lower the pin
   2. If second the second half of a bit, if sending zero, lower pin,
      If sending one, raise the pin,
   3. After both halves of the bit, shift the transmit register
   4. Once nothing is left to transmit, ensure the pin is low

To test the encoder, let's consider a one byte payload message,
whose encoded source is 0x12, dest is 0x34, and the payload
byte is 0x56

The total encoded message is 0xE1 0x12 0x34 0x56 which translates
to 1110 0001 0001 0010 0011 0100 0101 0110 and a transmit bit
count of (1*3) * 16 + 1 == 4 \* 16 + 1 == 65

We will use `_` to indicate a low, and `^` to indicate a high.

Transmission always starts low

The message is sent as:

```
   1 1 1 0 0 0 0 1 0 0 0 1 0 0 1 0 0 0 1 1 0 1 0 0 0 1 0 1 0 1 1 0
___^_^_^^_^_^_^__^^_^_^__^^_^__^^_^_^__^_^^__^^_^_^__^^__^^__^_^^_^
preamble          source --------|                payload  ------|
        length --|                destination ---|                "end"
```

If we were to receive that message, we'd actually receive it in
terms of a start signal, and then bit times of 1, 1.5, 2, or >2

Thus thus the sequence we'd see would be:

1. Start
2. 1t twice
3. 1.5t once
4. 1t twice
5. 1.5t twice
6. 1t once
7. 1.5t four times
8. 1t once
9. 1.5t once
10. 1t once
11. 2t once
12. 1.5t once
13. 1t once.
14. 1.5t once
15. 2t twice
16. 1t once
17. 1.5t (or timeout, depending on "end")

Decoding that timing sequence gives us:

```
1110 0001 0001 0010 0011 0100 0101 0110
```

Which demonstrates that our time based decoder and our
edge encoder work correctly together.

# STM32F031C6 setup (proto1)

The input pin isn't too important so long as it avoids
colliding with SPI or timer interrupts.

The output pin should be one that SPI1 can drive, so
one of PA7, PB5, PB15
