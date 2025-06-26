/** @type {import('tailwindcss').Config} */
module.exports = {
  mode: "all",
  content: ["./src/**/*.{rs,html,css}", "./dist/**/*.html"],
  theme: {
    extend: {
      colors: {
        // macOS System Colors
        macos: {
          // Light mode colors
          background: '#ffffff',
          surface: '#f5f5f7',
          card: '#ffffff',
          border: '#d2d2d7',
          'border-focus': '#007aff',
          text: '#1d1d1f',
          'text-secondary': '#6e6e73',
          'text-tertiary': '#8e8e93',
          
          // macOS Blue (System Blue)
          blue: {
            50: '#e5f3ff',
            100: '#cce6ff',
            200: '#99cdff',
            300: '#66b5ff',
            400: '#339cff',
            500: '#007aff', // Primary macOS blue
            600: '#0056cc',
            700: '#003d99',
            800: '#002966',
            900: '#001433',
          },
          
          // macOS Gray (System Gray)
          gray: {
            50: '#fafafa',
            100: '#f5f5f7',
            200: '#e5e5ea',
            300: '#d2d2d7',
            400: '#aeaeb2',
            500: '#8e8e93',
            600: '#6e6e73',
            700: '#48484a',
            800: '#2c2c2e',
            900: '#1c1c1e',
          },
          
          // macOS Green
          green: {
            500: '#30d158',
            600: '#28cd41',
          },
          
          // macOS Red
          red: {
            500: '#ff3b30',
            600: '#d70015',
          },
          
          // macOS Orange
          orange: {
            500: '#ff9500',
            600: '#ff8c00',
          },
          
          // macOS Yellow
          yellow: {
            500: '#ffcc00',
            600: '#ffb000',
          },
          
          // macOS Purple
          purple: {
            500: '#af52de',
            600: '#9542be',
          },
          
          // Sidebar colors
          sidebar: {
            background: '#f5f5f7',
            'background-hover': '#e5e5ea',
            'background-selected': '#007aff',
            text: '#1d1d1f',
            'text-secondary': '#6e6e73',
            'text-selected': '#ffffff',
          },
          
          // Button colors
          button: {
            primary: '#007aff',
            'primary-hover': '#0056cc',
            secondary: '#f5f5f7',
            'secondary-hover': '#e5e5ea',
            'secondary-border': '#d2d2d7',
          }
        }
      },
      fontFamily: {
        'sf': ['-apple-system', 'BlinkMacSystemFont', 'SF Pro Display', 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'sans-serif'],
        'sf-mono': ['SF Mono', 'Monaco', 'Inconsolata', 'Roboto Mono', 'monospace'],
      },
      borderRadius: {
        'macos': '10px',
        'macos-lg': '12px',
        'macos-xl': '16px',
      },
      boxShadow: {
        'macos': '0 2px 8px rgba(0, 0, 0, 0.08)',
        'macos-lg': '0 4px 16px rgba(0, 0, 0, 0.12)',
        'macos-xl': '0 8px 32px rgba(0, 0, 0, 0.16)',
        'macos-focus': '0 0 0 3px rgba(0, 122, 255, 0.2)',
      },
      backdropBlur: {
        'macos': '20px',
      },
      spacing: {
        'macos-xs': '4px',
        'macos-sm': '8px',
        'macos-md': '12px',
        'macos-lg': '16px',
        'macos-xl': '24px',
        'macos-2xl': '32px',
      },
      animation: {
        'macos-bounce': 'macos-bounce 0.3s ease-out',
        'macos-slide-in': 'macos-slide-in 0.2s ease-out',
      },
    },
  },
  plugins: [],
};
