import { defineConfig } from 'astro/config';
import solid from '@astrojs/solid-js';
import tailwind from '@astrojs/tailwind';

export default defineConfig({
  integrations: [solid(), tailwind()],
  vite: {
    server: {
      proxy: {
        '/api': {
          target: 'http://localhost:3000',
          changeOrigin: true,
          secure: false,
        },
      },
    },
  },
});
