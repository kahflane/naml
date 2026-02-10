import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { readFileSync } from 'fs';

const namlGrammar = JSON.parse(
  readFileSync('./src/shiki-languages/naml.tmLanguage.json', 'utf-8')
);

export default defineConfig({
  integrations: [
    starlight({
      title: 'naml',
      description: 'A fast, cross-platform programming language',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/kahflane/naml' },
      ],
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        { label: 'Guide', autogenerate: { directory: 'guide' } },
        { label: 'Language', autogenerate: { directory: 'language' } },
        { label: 'Standard Library', autogenerate: { directory: 'stdlib' } },
        { label: 'Examples', autogenerate: { directory: 'examples' } },
      ],
    }),
  ],
  markdown: {
    shikiConfig: {
      langs: [namlGrammar],
    },
  },
});
