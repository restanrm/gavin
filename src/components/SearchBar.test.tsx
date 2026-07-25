import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SearchBar } from '../components/SearchBar';

describe('SearchBar', () => {
  it('renders with placeholder', () => {
    render(<SearchBar value="" onChange={() => {}} placeholder="Search here" />);
    const input = screen.getByPlaceholderText('Search here');
    expect(input).toBeInTheDocument();
  });

  it('displays the current value', () => {
    render(<SearchBar value="test query" onChange={() => {}} />);
    const input = screen.getByRole('searchbox');
    expect(input).toHaveValue('test query');
  });

  it('calls onChange when user types', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    
    render(<SearchBar value="" onChange={handleChange} />);
    const input = screen.getByRole('searchbox');

    await user.type(input, 'Test');

    // Should be called when user types
    expect(handleChange).toHaveBeenCalled();
    expect(handleChange.mock.calls.length).toBeGreaterThan(0);
  });

  it('has accessible label', () => {
    render(<SearchBar value="" onChange={() => {}} />);
    const input = screen.getByLabelText(/search vinyls/i);
    expect(input).toBeInTheDocument();
  });
});
