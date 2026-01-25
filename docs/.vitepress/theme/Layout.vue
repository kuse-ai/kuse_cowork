<script setup>
import DefaultTheme from 'vitepress/theme'
import { useData } from 'vitepress'

const { Layout } = DefaultTheme
const { frontmatter } = useData()
</script>

<template>
  <Layout>
    <template #layout-bottom>
      <div class="made-with-kuse">
        <a href="https://www.kuse.ai/" target="_blank" rel="noopener noreferrer">
          made with kuse
        </a>
      </div>
    </template>
  </Layout>
</template>

<style scoped>
.made-with-kuse {
  display: flex;
  justify-content: center;
  padding: 48px 0 64px;
}

.made-with-kuse a {
  font-family: 'Poppins', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 14px;
  font-weight: 400;
  color: var(--vp-c-text-2);
  text-decoration: none;
  transition: color 0.2s ease;
}

.made-with-kuse a:hover {
  color: var(--vp-c-brand-1);
}
</style>
