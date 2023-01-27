# praw v1: compressed streaming audio made easy

A new file format to store compressed audio in stacks to stream to any device. .

## How it works

All of the following documentation is in `BE` (Big Endian) byte order.

## V1

### Header

0. (IF FILE)
- Magic number to know if it's a praw file or not. This will be "praw" in ASCII. This is 4 bytes long.
- The first 4 bytes are a BE int that indicates the length of the header in bytes. It is dynamic.
1. The first 4 bytes are a BE int that indicates the version of the file format. This is currently `1`.
2. The next 4 bytes are a BE int that indicates de sample rate of the original PCM.
3. The next 4 bytes indicate the amount of tracks that the packs contain.
4. The next bytes are 0 if track is mono, 1 if track is stereo. The first byte is the first track, the second byte is the second track, and so on. The last byte is the last track.
5. The next byte is type of audio container: 0 for opus, 1 for flac.
6. The next 4 bytes are a BE int that indicates the number of praw packs in the file.

### Packs

1. The first 4 bytes are a BE int that indicates the length of the pack in bytes.
2. The next bytes are the length of each track. Each integer is 4 bytes long and is a BE int. The first integer is the length of the first track, the second integer is the length of the second track, and so on. The last integer is the length of the last track.
3. The pack contains the opus audio of each track. The first track is the first track, the second track is the second track, and so on. The last track is the last track. Length of the pack is indicated in the bytes before.
