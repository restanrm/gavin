import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useDebounce } from '../hooks/useDebounce';

describe('useDebounce', () => {
  it('returns initial value immediately', () => {
    const { result } = renderHook(() => useDebounce('initial', 300));
    expect(result.current).toBe('initial');
  });

  it('debounces value changes', async () => {
    const { result, rerender } = renderHook(
      ({ value, delay }) => useDebounce(value, delay),
      { initialProps: { value: 'first', delay: 100 } }
    );

    expect(result.current).toBe('first');

    // Update value
    rerender({ value: 'second', delay: 100 });
    expect(result.current).toBe('first'); // Still old value

    // Wait for debounce
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 150));
    });

    expect(result.current).toBe('second');
  });

  it('handles rapid changes', async () => {
    const { result, rerender } = renderHook(
      ({ value, delay }) => useDebounce(value, delay),
      { initialProps: { value: 'first', delay: 100 } }
    );

    // Rapidly change value
    rerender({ value: 'second', delay: 100 });
    await new Promise((resolve) => setTimeout(resolve, 50));
    rerender({ value: 'third', delay: 100 });
    await new Promise((resolve) => setTimeout(resolve, 50));
    rerender({ value: 'fourth', delay: 100 });

    // Still showing initial
    expect(result.current).toBe('first');

    // Wait for final debounce
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 150));
    });

    // Should show last value
    expect(result.current).toBe('fourth');
  });
});
