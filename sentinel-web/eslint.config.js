import js from "@eslint/js";
import tseslint from "typescript-eslint";
import pluginVue from "eslint-plugin-vue";
import globals from "globals";

export default tseslint.config(
  {
    // Fichiers/dossiers ignores globalement.
    ignores: ["dist/**", "node_modules/**", "*.config.*", "coverage/**"],
  },

  // Base JS + TS recommandes.
  js.configs.recommended,
  ...tseslint.configs.recommended,

  // Vue (flat/recommended) pour les .vue.
  ...pluginVue.configs["flat/recommended"],

  // Le parser Vue doit utiliser le parser TS pour <script setup lang="ts">.
  {
    files: ["**/*.vue"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },

  // Reglages communs : env navigateur + assouplissements pour ne pas
  // bloquer la CI sur le code existant (guardrail, pas un mur).
  {
    files: ["**/*.{ts,tsx,vue,js}"],
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
    rules: {
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": "warn",
      // `{}` est utilise dans vite-env.d.ts (shims Vue) : guardrail, pas un mur.
      "@typescript-eslint/no-empty-object-type": "warn",
      "no-console": ["warn", { allow: ["error", "warn"] }],

      // v-slot dynamique existant (DataTable) : abaisse en warn.
      "vue/valid-v-slot": "warn",

      "vue/multi-word-component-names": "off",
      // On garde en "warn" les regles a portee semantique / convention.
      "vue/require-default-prop": "warn",
      "vue/order-in-components": "warn",
      "vue/attribute-hyphenation": "warn",
      "vue/v-on-event-hyphenation": "warn",
      // Regles purement cosmetiques (formatage) desactivees : elles relevent
      // d'un formateur (Prettier), pas du linter, et noyaient les vrais
      // signaux sous ~1750 warnings. Le guardrail se concentre sur le fond.
      "vue/attributes-order": "off",
      "vue/html-self-closing": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/multiline-html-element-content-newline": "off",
      "vue/html-closing-bracket-spacing": "off",
      "vue/max-attributes-per-line": "off",
      "vue/html-indent": "off",
      "vue/html-closing-bracket-newline": "off",
      "vue/first-attribute-linebreak": "off",
    },
  },
);
