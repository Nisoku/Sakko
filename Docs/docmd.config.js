module.exports = {
  siteTitle: 'Sakko',
  srcDir: 'docs',
  outputDir: 'site',

  search: true,
  autoTitleFromH1: true,
  copyCode: true,

  theme: {
    name: 'sky',
    defaultMode: 'system',
    enableModeToggle: true,
  },

  navigation: [
    { title: 'Home', path: '/', icon: 'home' },
    {
      title: 'Guide',
      icon: 'book',
      collapsible: true,
      children: [
        { title: 'Language Reference', path: '/language-reference/' },
      ],
    },
    { title: 'API Reference', path: '/api-reference/', icon: 'code' },
  ],

  footer: 'Sakko. Apache License v2.0.',
};
