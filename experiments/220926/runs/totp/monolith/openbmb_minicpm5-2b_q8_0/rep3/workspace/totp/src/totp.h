#define TOTP_OK 0
#define TOTP_EBOUNDS -1

/* convert u32 to 4 bytes, big-endian */
static inline uint32_t
unpack32(uint32_t x, uint8_t a[4])
{
	a[0] = (uint8_t)(x >> 24);
	a[1] = (uint8_t)(x >> 16);
	a[2] = (uint8_t)(x >> 8);
	a[3] = (uint8_t)x;
}

/* convert u64 to 8 bytes, big-endian */
static inline uint32_t
unpack64(uint64_t x, uint8_t a[8])
{
	unpack32((uint32_t)(x >> 32), &a[0]);
	unpack32((uint32_t)x, &a[4]);
}

/* convert 4 bytes to u32, big-endian */
static inline uint32_t
pack32(const uint8_t a[4])
{
	return (a[0] << 24) | (a[1] << 16) | (a[2] << 8) | a[3];
}

/* FIPS 180-3 2.2.2 */
static inline uint32_t
rotl(uint32_t x, int n)
{
	return x << n | x >> (32-n);
}
