// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'kit',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/ahappypie/builderkit' }],
			sidebar: [
				{
					label: 'Guides',
					items: [
						{
							label: 'Getting started',
							items: [
								{ label: 'Overview', slug: 'guides/get_started' },
								{ label: 'macOS', slug: 'guides/get_started/macos' },
								{ label: 'Windows', slug: 'guides/get_started/windows' },
							],
						},
					],
				},
				{
					label: 'Reference',
					items: [{ autogenerate: { directory: 'reference' } }],
				},
			],
			customCss: [
				"./src/styles/brand.css",
				// "./src/styles/layout.css",
				// "./src/styles/color.css",
				// "./src/styles/font.css",
				"./src/styles/hero.css",
				"./src/styles/terminal.css",
				// "./src/styles/typography.css",
			]
		}),
	],
});
