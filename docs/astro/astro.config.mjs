// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	site: 'https://serenade.rs',
	base: '/docs',
	trailingSlash: 'always',
	integrations: [
		starlight({
			title: 'Serenade',
			description: 'Guides and reference for the Serenade Rust application framework.',
			favicon: '/favicon-32.png',
			disable404Route: true,
			components: {
				SiteTitle: './src/components/SiteTitle.astro',
				Sidebar: './src/components/Sidebar.astro',
				Footer: './src/components/Footer.astro',
			},
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com/Interchouette-ITC/Serenade',
				},
			],
			customCss: ['./src/styles/serenade.css'],
			sidebar: [
				{
					label: 'Start here',
					items: [
						{ label: 'Installation', slug: 'first-steps/installation' },
						{ label: 'Run the demo application', slug: 'first-steps/first-application' },
						{ label: 'Choose a web mode', slug: 'first-steps/choose-your-mode' },
					],
				},
				{
					label: 'Tutorials',
					items: [{ autogenerate: { directory: 'tutorials' } }],
				},
				{
					label: 'Concepts',
					items: [{ autogenerate: { directory: 'concepts' } }],
				},
				{
					label: 'How-to guides',
					items: [{ autogenerate: { directory: 'how-to' } }],
				},
				{
					label: 'Components',
					items: [
						{ slug: 'components' },
						{ slug: 'components/catalog' },
					],
				},
				{
					label: 'Examples',
					items: [{ autogenerate: { directory: 'examples' } }],
				},
				{
					label: 'Reference',
					items: [{ autogenerate: { directory: 'reference' } }],
				},
			],
		}),
	],
});
