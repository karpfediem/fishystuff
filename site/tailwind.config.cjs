/** @type {import('tailwindcss').Config} */
module.exports = {
    // Support both strategies:
    //  - class="dark"
    //  - data-theme="dark" (works well with daisyUI)
    darkMode: ['class', '[data-theme="dark"]'],

    // What Tailwind should scan for class names
    content: [
        './layouts/**/*.{shtml,html}',
        './content/**/*.smd',
        './assets/**/*.js',

        // Optional: include built output for exact pruning (enable if you use .out)
        // './.out/**/*.html',
    ],

    // TEMP: ship everything while you stabilize templates
    // Remove or replace with a narrower set once you’re confident
    safelist: [{ pattern: /.*/ }],

    theme: {
        extend: {
            fontSize: {
                base: '1.5rem',
            },
        },
    },

    plugins: [
        require('daisyui'),
        require('@tailwindcss/typography'),
    ],

    // DaisyUI basic theme setup (adjust later if you want)
    daisyui: {
        themes: ['light', 'dark'],
    },
};
