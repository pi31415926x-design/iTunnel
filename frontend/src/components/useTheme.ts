import { ref } from "vue";

const isDark = ref(false);

export function useTheme() {
  // 初始化
  const initTheme = () => {
    const saved = localStorage.getItem("theme");
    isDark.value = saved === "dark";
    applyTheme();
  };

  const applyTheme = () => {
    const root = document.documentElement;
    if (isDark.value) {
      root.classList.add("dark");
      localStorage.setItem("theme", "dark");
    } else {
      root.classList.remove("dark");
      localStorage.setItem("theme", "light");
    }
  };

  const toggleTheme = () => {
    isDark.value = !isDark.value;
    applyTheme();
  };

  return {
    isDark,
    initTheme,
    toggleTheme,
  };
}
