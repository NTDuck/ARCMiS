#include "std.h"

typedef unsigned char uint8_t;
typedef unsigned int uint32_t;
typedef unsigned long long uint64_t;
typedef unsigned long size_t;

#define SIZE_MAX (~(size_t)0)

void *
memset(void *s, int c, size_t n)
{
	size_t i;

	for (i = 0; i < n; i++)
		((uint8_t *)s)[i] = (uint8_t)c;

	return s;
}

void *
memcpy(void *dst, const void *src, size_t n)
{
	size_t i;

	for (i = 0; i < n; i++)
		((uint8_t *)dst)[i] = ((uint8_t *)src)[i];

	return dst;
}

size_t
strlen(const char *s)
{
	size_t i;

	for (i = 0; s[i]; i++) ;

	return i;
}
