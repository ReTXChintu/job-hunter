import { createSystem, defaultConfig, defineConfig } from "@chakra-ui/react";

const config = defineConfig({
  globalCss: {
    "html, body, #root": {
      height: "100%",
      bg: "bg",
      color: "fg",
      overflow: "hidden",
    },
    body: {
      fontFamily: "body",
      fontSize: "sm",
      userSelect: "none",
    },
    "input, textarea, [contenteditable], .selectable": {
      userSelect: "text",
    },
    "::-webkit-scrollbar": { width: "10px", height: "10px" },
    "::-webkit-scrollbar-thumb": { bg: "border", borderRadius: "full", border: "2px solid transparent", backgroundClip: "content-box" },
    "::-webkit-scrollbar-track": { bg: "transparent" },
  },
  theme: {
    tokens: {
      fonts: {
        body: { value: `"Segoe UI Variable", "Segoe UI", system-ui, -apple-system, "Inter", sans-serif` },
        heading: { value: `"Segoe UI Variable", "Segoe UI", system-ui, -apple-system, "Inter", sans-serif` },
        mono: { value: `"Cascadia Code", "JetBrains Mono", "SF Mono", Consolas, monospace` },
      },
      colors: {
        brand: {
          50: { value: "#eef2ff" },
          100: { value: "#e0e7ff" },
          200: { value: "#c7d2fe" },
          300: { value: "#a5b4fc" },
          400: { value: "#818cf8" },
          500: { value: "#6366f1" },
          600: { value: "#4f46e5" },
          700: { value: "#4338ca" },
          800: { value: "#3730a3" },
          900: { value: "#312e81" },
          950: { value: "#1e1b4b" },
        },
      },
      radii: {
        sm: { value: "4px" },
        md: { value: "6px" },
        lg: { value: "8px" },
      },
    },
    semanticTokens: {
      colors: {
        brand: {
          solid: { value: { _light: "{colors.brand.600}", _dark: "{colors.brand.400}" } },
          contrast: { value: { _light: "white", _dark: "{colors.brand.950}" } },
          fg: { value: { _light: "{colors.brand.700}", _dark: "{colors.brand.300}" } },
          muted: { value: { _light: "{colors.brand.100}", _dark: "{colors.brand.900}" } },
          subtle: { value: { _light: "{colors.brand.50}", _dark: "{colors.brand.950}" } },
          emphasized: { value: { _light: "{colors.brand.200}", _dark: "{colors.brand.800}" } },
          focusRing: { value: { _light: "{colors.brand.500}", _dark: "{colors.brand.400}" } },
        },
        bg: {
          sidebar: { value: { _light: "#f6f7fb", _dark: "#0f1117" } },
          canvas: { value: { _light: "#ffffff", _dark: "#14161d" } },
        },
      },
    },
  },
});

export const system = createSystem(defaultConfig, config);
