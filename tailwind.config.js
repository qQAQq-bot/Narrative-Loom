/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Fabric theme - warm, natural tones (light mode defaults, dark mode via CSS)
        // Using rgb format to support opacity modifiers like /40
        fabric: {
          linen: 'rgb(var(--fabric-linen) / <alpha-value>)',
          canvas: 'rgb(var(--fabric-canvas) / <alpha-value>)',
          denim: 'rgb(var(--fabric-denim) / <alpha-value>)',
          thread: 'rgb(var(--fabric-thread) / <alpha-value>)',
          cream: 'rgb(var(--fabric-cream) / <alpha-value>)',
          sand: 'rgb(var(--fabric-sand) / <alpha-value>)',
          warm: 'rgb(var(--fabric-warm) / <alpha-value>)',
          sepia: 'rgb(var(--fabric-sepia) / <alpha-value>)',
        },
        // Primary - warm book theme
        primary: {
          50: '#faf8f5',
          100: '#f0ebe3',
          200: '#e0d5c5',
          300: '#c9b89a',
          400: '#b39a70',
          500: '#9a7d50',
          600: '#7d6340',
          700: '#5f4a30',
          800: '#423322',
          900: '#2a2015',
          950: '#1a130d',
        },
        // Accent colors for different card types
        accent: {
          technique: '#6b8cae',   // 技法 - Muted blue
          character: '#7fa67f',   // 人物 - Sage green
          setting: '#9b7bb8',     // 设定 - Dusty purple
          event: '#c4956a',       // 事件 - Terracotta
          timeline: '#b87d8c',    // 时间线 - Dusty rose
        },
      },
      fontFamily: {
        serif: ['Noto Serif SC', 'Source Han Serif SC', 'Georgia', 'serif'],
        sans: ['Inter', 'Noto Sans SC', 'system-ui', 'sans-serif'],
      },
      spacing: {
        '18': '4.5rem',
        '88': '22rem',
        '128': '32rem',
      },
      borderRadius: {
        '4xl': '2rem',
      },
      boxShadow: {
        'fabric': '0 2px 8px -2px var(--fabric-shadow), 0 4px 16px -4px var(--fabric-shadow)',
        'fabric-lg': '0 4px 12px -2px var(--fabric-shadow-strong), 0 8px 24px -4px var(--fabric-shadow)',
        'fabric-inner': 'inset 0 1px 2px var(--fabric-shadow)',
        'stitch': '0 0 0 2px rgba(139, 115, 85, 0.1)',
      },
      backgroundImage: {
        'linen': `url("data:image/svg+xml,%3Csvg width='60' height='60' viewBox='0 0 60 60' xmlns='http://www.w3.org/2000/svg'%3E%3Cg fill='none' fill-rule='evenodd'%3E%3Cg fill='%23d4c4a8' fill-opacity='0.15'%3E%3Cpath d='M36 34v-4h-2v4h-4v2h4v4h2v-4h4v-2h-4zm0-30V0h-2v4h-4v2h4v4h2V6h4V4h-4zM6 34v-4H4v4H0v2h4v4h2v-4h4v-2H6zM6 4V0H4v4H0v2h4v4h2V6h4V4H6z'/%3E%3C/g%3E%3C/g%3E%3C/svg%3E")`,
        'canvas': `url("data:image/svg+xml,%3Csvg width='40' height='40' viewBox='0 0 40 40' xmlns='http://www.w3.org/2000/svg'%3E%3Cg fill='%23c9b89a' fill-opacity='0.08'%3E%3Cpath d='M0 0h20v20H0V0zm20 20h20v20H20V20z'/%3E%3C/g%3E%3C/svg%3E")`,
        'weave': `url("data:image/svg+xml,%3Csvg width='8' height='8' viewBox='0 0 8 8' xmlns='http://www.w3.org/2000/svg'%3E%3Cg fill='%238b7355' fill-opacity='0.05'%3E%3Cpath d='M0 0h4v4H0V0zm4 4h4v4H4V4z'/%3E%3C/g%3E%3C/svg%3E")`,
      },
      animation: {
        'fade-in': 'fadeIn 0.22s ease-out',
        'slide-up': 'slideUp 0.22s ease-out',
        'slide-in-right': 'slideInRight 0.22s ease-out',
        'lift': 'lift 0.2s ease-out forwards',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(8px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        slideInRight: {
          '0%': { opacity: '0', transform: 'translateX(8px)' },
          '100%': { opacity: '1', transform: 'translateX(0)' },
        },
        lift: {
          '0%': { transform: 'translateY(0)', boxShadow: '0 2px 8px -2px rgba(139, 115, 85, 0.15)' },
          '100%': { transform: 'translateY(-2px)', boxShadow: '0 6px 16px -4px rgba(139, 115, 85, 0.25)' },
        },
      },
      transitionDuration: {
        '180': '180ms',
        '220': '220ms',
        '260': '260ms',
      },
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
    require('@tailwindcss/forms'),
  ],
}
