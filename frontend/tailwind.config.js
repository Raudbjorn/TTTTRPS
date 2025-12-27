/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{rs,html,css}",
    "./dist/**/*.html",
    "./index.html"
  ],
  theme: {
    extend: {
      colors: {
        theme: {
          primary: 'var(--bg-primary)',
          secondary: 'var(--bg-secondary)',
          accent: 'var(--accent)',
          text: 'var(--text-primary)',
          muted: 'var(--text-muted)',
          border: 'var(--border-color)',
        }
      }
    },
  },
  plugins: [],
}
