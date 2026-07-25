import type { ThemePreference } from '../hooks/useTheme';
import { useTheme } from '../hooks/useTheme';

const THEME_OPTIONS: Array<{
  value: ThemePreference;
  label: string;
  icon: string;
}> = [
  { value: 'light', label: 'Light', icon: '☀️' },
  { value: 'dark', label: 'Dark', icon: '🌙' },
  { value: 'system', label: 'System', icon: '💻' },
];

export function ThemeToggle() {
  const { preference, resolvedTheme, setPreference } = useTheme();

  return (
    <div
      className="theme-toggle"
      role="group"
      aria-label={`Color theme, currently ${preference} (${resolvedTheme})`}
    >
      {THEME_OPTIONS.map((option) => (
        <button
          key={option.value}
          type="button"
          className={`theme-toggle-option${preference === option.value ? ' active' : ''}`}
          aria-pressed={preference === option.value}
          onClick={() => setPreference(option.value)}
          title={`${option.label} theme`}
        >
          <span aria-hidden="true">{option.icon}</span>
          <span className="theme-toggle-label">{option.label}</span>
        </button>
      ))}
    </div>
  );
}
