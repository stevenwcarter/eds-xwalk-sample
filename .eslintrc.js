module.exports = {
  root: true,
  extends: [
    'airbnb-base',
    'plugin:json/recommended',
    'plugin:xwalk/recommended',
  ],
  env: {
    browser: true,
  },
  parser: '@babel/eslint-parser',
  parserOptions: {
    allowImportExportEverywhere: true,
    sourceType: 'module',
    requireConfigFile: false,
  },
  rules: {
    'import/extensions': ['error', { js: 'always' }], // require js file extensions in imports
    'linebreak-style': ['error', 'unix'], // enforce unix linebreaks
    'no-param-reassign': [2, { props: false }], // allow modifying properties of param
    // `fieldsdemo` is a field-type test harness, not a content block: it exists to
    // exercise every Universal Editor component type in one place, so the 4-cell
    // content-modelling guidance does not apply. Every other model stays at 4.
    'xwalk/max-cells': ['error', { fieldsdemo: 100 }],
  },
  overrides: [
    {
      // ledge plugin panels run in the browser and import the panel SDK from a URL
      // the ledge server serves at runtime (`/plugins/_sdk/panel-runtime.js`); there
      // is no such file on disk, and letting eslint --fix rewrite it to a relative
      // path would break the panel. See plugins/gallery-panel/panel/index.js.
      files: ['plugins/**/*.js'],
      rules: {
        'import/no-absolute-path': 'off',
        'import/no-unresolved': 'off',
      },
    },
  ],
};
