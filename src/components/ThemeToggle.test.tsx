import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeToggle } from './ThemeToggle';

describe('ThemeToggle', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.colorScheme = '';
  });

  afterEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.colorScheme = '';
  });

  it('uses the system theme by default', () => {
    render(<ThemeToggle />);

    expect(screen.getByRole('button', { name: /system/i })).toHaveAttribute('aria-pressed', 'true');
    expect(document.documentElement).not.toHaveAttribute('data-theme');
  });

  it('stores and applies an explicit dark theme', async () => {
    const user = userEvent.setup();
    render(<ThemeToggle />);

    await act(async () => {
      await user.click(screen.getByRole('button', { name: /dark/i }));
    });

    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
    expect(window.localStorage.getItem('gavin-theme')).toBe('dark');
  });
});
