import unittest
import emu_8085

class TestCore(unittest.TestCase):
    """Unit tests for the core emu_8085 datatypes (bits, DataSize, Data, Mask, MaskedData)."""

    def test_bits_utility(self):
        # bits(n) should return (2^n - 1)
        self.assertEqual(emu_8085.core.bits(0), 0)
        self.assertEqual(emu_8085.core.bits(1), 1)
        self.assertEqual(emu_8085.core.bits(4), 15)  # 0b1111
        self.assertEqual(emu_8085.core.bits(8), 255)  # 0xFF

    def test_datasize_enum(self):
        self.assertEqual(emu_8085.DataSize.BYTE, 8)
        self.assertEqual(emu_8085.DataSize.WORD, 16)
        self.assertEqual(str(emu_8085.DataSize.BYTE), "BYTE")
        self.assertEqual(repr(emu_8085.DataSize.WORD), "DataSize(WORD)")

    def test_data_initializers(self):
        # Test basic byte/word/etc constructors
        d_bit_on = emu_8085.Data.on()
        self.assertEqual(d_bit_on.value, 1)
        self.assertEqual(d_bit_on.size, emu_8085.DataSize.BIT)

        d_bit_off = emu_8085.Data.off()
        self.assertEqual(d_bit_off.value, 0)
        self.assertEqual(d_bit_off.size, emu_8085.DataSize.BIT)

        d_ch = emu_8085.Data.ch('A')
        self.assertEqual(d_ch.value, 65)
        self.assertEqual(d_ch.size, emu_8085.DataSize.BYTE)

        d_ch_bytes = emu_8085.Data.ch(b'B')
        self.assertEqual(d_ch_bytes.value, 66)

        d_byte = emu_8085.Data.byte(0x1FF)  # should mask to 0xFF
        self.assertEqual(d_byte.value, 0xFF)
        self.assertEqual(d_byte.size, emu_8085.DataSize.BYTE)

        d_words = emu_8085.Data.words(0x12, 0x34)
        self.assertEqual(d_words.value, 0x1234)
        self.assertEqual(d_words.size, emu_8085.DataSize.WORD)

        d_word = emu_8085.Data.word(0x12345)  # should mask to 0x2345
        self.assertEqual(d_word.value, 0x2345)

        d_dwords = emu_8085.Data.dwords(0x12, 0x34, 0x56)
        self.assertEqual(d_dwords.value, 0x123456)
        self.assertEqual(d_dwords.size, emu_8085.DataSize.DWORD)

        d_dword = emu_8085.Data.dword(0x123456789)  # mask to 0x23456789
        self.assertEqual(d_dword.value, 0x23456789)

        d_qwords = emu_8085.Data.qwords(0x12, 0x34, 0x56, 0x78)
        self.assertEqual(d_qwords.value, 77312841336)
        self.assertEqual(d_qwords.size, emu_8085.DataSize.QWORD)

        d_qword = emu_8085.Data.qword(0xFFFFFFFFFFFFFFFF)
        self.assertEqual(d_qword.value, 0xFFFFFFFFFFFFFFFF)

    def test_data_byte_extraction_and_reversing(self):
        d = emu_8085.Data.words(0x12, 0x34)
        # Big-endian internally: value is 0x1234
        # Byte at 0 (least significant): 0x34
        # Byte at 1: 0x12
        self.assertEqual(d.byte_at(0), 0x34)
        self.assertEqual(d.byte_at(1), 0x12)
        self.assertEqual(d[0], 0x34)
        self.assertEqual(d[1], 0x12)

        # Reversing bytes: 0x1234 (WORD) -> 0x3412
        d_rev = d.reverse()
        self.assertEqual(d_rev.value, 0x3412)
        self.assertEqual(d_rev.size, emu_8085.DataSize.WORD)

        # Bit level reverse (size < 8) should keep value
        bit_d = emu_8085.Data(5, emu_8085.DataSize.NIBBLE)
        self.assertEqual(bit_d.reverse().value, 5)

    def test_data_conversions_and_arithmetics(self):
        d1 = emu_8085.Data.byte(10)
        d2 = emu_8085.Data.byte(20)

        # Size adjustment
        self.assertEqual(d1.to_size(emu_8085.DataSize.WORD).size, emu_8085.DataSize.WORD)

        # Arithmetics
        self.assertEqual((d1 + d2).value, 30)
        self.assertEqual((d2 - d1).value, 10)
        self.assertEqual((d1 * d2).value, 200)
        self.assertEqual((d1 + 5).value, 15)
        self.assertEqual((5 + d1).value, 15)
        self.assertEqual((d2 - 5).value, 15)
        self.assertEqual((25 - d2).value, 5)
        self.assertEqual((d1 * 2).value, 20)
        self.assertEqual((2 * d1).value, 20)

        # Bitwise operators
        d_bin1 = emu_8085.Data.byte(0b1100)
        d_bin2 = emu_8085.Data.byte(0b1010)
        self.assertEqual((d_bin1 & d_bin2).value, 0b1000)
        self.assertEqual((d_bin1 | d_bin2).value, 0b1110)
        self.assertEqual((d_bin1 ^ d_bin2).value, 0b0110)
        self.assertEqual((d_bin1 & 0b1010).value, 0b1000)
        self.assertEqual((0b1010 & d_bin1).value, 0b1000)
        self.assertEqual((d_bin1 | 0b1010).value, 0b1110)
        self.assertEqual((0b1010 | d_bin1).value, 0b1110)
        self.assertEqual((d_bin1 ^ 0b1010).value, 0b0110)
        self.assertEqual((0b1010 ^ d_bin1).value, 0b0110)

        # Invert, shifts, int conversion, representations
        self.assertEqual((~emu_8085.Data.byte(0x0F)).value, 0xF0)
        self.assertEqual((emu_8085.Data.byte(1) << 2).value, 4)
        self.assertEqual((emu_8085.Data.byte(4) >> 2).value, 1)
        self.assertEqual(int(d1), 10)
        self.assertEqual(d1, 10)
        self.assertEqual(d1, emu_8085.Data.byte(10))
        self.assertNotEqual(d1, "string")
        self.assertEqual(bytes(emu_8085.Data.byte(0xAB)), b'\xab')

        # Test unknown size bytes formatting
        unknown_d = emu_8085.Data(0x1234, emu_8085.DataSize.UNKNOWN)
        self.assertEqual(bytes(unknown_d), b'\x12\x34')
        self.assertIn("Data", repr(d1))

    def test_mask(self):
        # Mask of 4 bits with offset 2 (should be 0b111100 = 60)
        mask = emu_8085.core.Mask.bits(4, 2)
        self.assertEqual(mask.value, 60)
        self.assertEqual(mask.offset, 2)
        self.assertEqual(len(mask), 6)
        self.assertEqual(mask.bit_count(), 4)
        self.assertEqual(repr(mask), "Mask(value=11_1100, offset=2)")
        self.assertEqual(str(mask), "11_1100")

        # Apply mask
        d_val = emu_8085.Data(0b111111, emu_8085.DataSize.BYTE)
        # Shifted (default): (0b111111 & 0b111100) >> 2 = 0b1111 (15)
        self.assertEqual(mask.apply(d_val).value, 15)
        # Non-shifted: 0b111111 & 0b111100 = 0b111100 (60)
        self.assertEqual(mask.apply(d_val, shift=False).value, 60)

    def test_masked_data(self):
        # 8-bit width masked data container
        md = emu_8085.core.MaskedData.byte(value=0x1FF)  # masks to 0xFF
        self.assertEqual(md.read().value, 0xFF)
        self.assertEqual(len(md), 8)
        self.assertEqual(md.bits, 8)
        self.assertEqual(str(md), "1111_1111")
        self.assertIn("Data", repr(md))

        # Check constructor factories
        self.assertEqual(emu_8085.core.MaskedData.bit(1).bits, 1)
        self.assertEqual(emu_8085.core.MaskedData.nibble(0xF).bits, 4)
        self.assertEqual(emu_8085.core.MaskedData.word(0xFFFF).bits, 16)
        self.assertEqual(emu_8085.core.MaskedData.dword(0xFFFFFFFF).bits, 32)
        self.assertEqual(emu_8085.core.MaskedData.qword(0xFFFFFFFFFFFFFFFF).bits, 64)
        self.assertEqual(emu_8085.core.MaskedData.bit_count(5).bits, 5)

        # Write safely with mask alignments
        md_reg = emu_8085.core.MaskedData.byte(0x00)
        # Sub-mask representing bits 2,3 (value = 12, offset = 2)
        sub_mask = emu_8085.core.Mask.bits(2, 2)
        # Write 3 to this sub-mask: (3 & 3) << 2 = 12 (0b1100)
        md_reg.write(3, sub_mask)
        self.assertEqual(md_reg.read().value, 12)
        # Write another value with no mask
        md_reg.write(0xAA)
        self.assertEqual(md_reg.read().value, 0xAA)
