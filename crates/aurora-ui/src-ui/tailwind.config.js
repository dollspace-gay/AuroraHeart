/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'deep-space': '#0B0C15',
        'panel-glass': 'rgba(19, 21, 36, 0.5)',
        'aurora-green': '#00FFB3',
        'aurora-green-alt': '#00FF9D',
        'nebula-purple': '#B24BF3',
        'glacial-blue': '#00D9FF',
        'text-white': '#E8E8E8',
        'text-dim': '#8892B0',
      },
      fontFamily: {
        'mono': ['"JetBrains Mono"', 'Consolas', 'Monaco', 'monospace'],
      },
      backdropBlur: {
        'glass': '24px',
      },
    },
  },
  plugins: [],
}
