<template>
  <footer
    v-if="show"
    class="absolute bottom-2 left-6 right-6 text-xs text-gray-400 flex justify-between"
  >
    <span>ARCMiS</span>
    <span>{{ section }}</span>
    <span>{{ $nav.currentSlideNo }} / {{ $nav.total }}</span>
  </footer>
</template>

<script setup>
import { computed } from 'vue'
import { useNav } from '@slidev/client'

const nav = useNav()

// Footer on every content slide: skip title (cover), dividers
// (section), and the closing slide (end).
const show = computed(() => !['cover', 'section', 'end'].includes(nav.currentLayout.value))

// Section name: title of the most recent section divider at or before
// the current slide.
const section = computed(() => {
  let name = 'ARCMiS'
  for (const slide of nav.slides.value) {
    const fm = slide.meta?.slide?.frontmatter || {}
    if (fm.layout !== 'section')
      continue
    if (slide.no <= nav.currentSlideNo.value)
      name = String(fm.title || name).trim()
  }
  return name
})
</script>
