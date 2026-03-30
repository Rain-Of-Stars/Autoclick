import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: "rgb(var(--color-surface) / <alpha-value>)",
        panel: "rgb(var(--color-panel) / <alpha-value>)",
        accent: "rgb(var(--color-accent) / <alpha-value>)",
        ink: "rgb(var(--color-ink) / <alpha-value>)",
        muted: "rgb(var(--color-muted) / <alpha-value>)",
        success: "rgb(var(--color-success) / <alpha-value>)",
        danger: "rgb(var(--color-danger) / <alpha-value>)",
        warning: "rgb(var(--color-warning) / <alpha-value>)"
      },
      boxShadow: {
        chrome: "0 18px 48px rgba(10, 18, 31, 0.35)"
      },
      borderRadius: {
        shell: "24px"
      },
      keyframes: {
        scan: {
          "0%, 100%": { backgroundPosition: "0% -100%" },
          "50%": { backgroundPosition: "0% 200%" }
        }
      },
      animation: {
        scan: "scan 12s linear infinite"
      }
    }
  },
  plugins: []
};

export default config;

