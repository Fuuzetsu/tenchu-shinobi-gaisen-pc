The STR AC Huffman table is adapted from `internal/media/mdec/huffman.go` in
https://github.com/bitwizeshift/psx-decomp-tools at 3c355886d023b03c4ddcf984f99d26c53916bef2.
Used under its MIT license, included here. No game resources are included.

Bitstream v3 and sector framing follow https://psx-spx.consoledev.net/cdromfileformats/;
spatial decoding follows https://psx-spx.consoledev.net/macroblockdecodermdec/.
The decoder is a presentation codec, not a cycle-accurate emulation of the MDEC.
