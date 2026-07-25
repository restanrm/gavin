import { describe, it, expect } from 'vitest';
import { parseCSV } from '../utils/csv';

describe('parseCSV', () => {
  it('parses valid CSV with all fields', () => {
    const csv = 'The Beatles,Abbey Road,1969,Final album,https://example.com/cover.jpg';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(1);
    expect(result.items[0]).toEqual({
      artist: 'The Beatles',
      title: 'Abbey Road',
      year: 1969,
      notes: 'Final album',
      cover_url: 'https://example.com/cover.jpg',
    });
  });

  it('parses CSV with only required fields', () => {
    const csv = 'Pink Floyd,The Wall';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(1);
    expect(result.items[0]).toEqual({
      artist: 'Pink Floyd',
      title: 'The Wall',
    });
  });

  it('parses multiple lines', () => {
    const csv = `The Beatles,Abbey Road,1969
Pink Floyd,The Wall,1979
Miles Davis,Kind of Blue,1959`;
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(3);
    expect(result.items[0]?.artist).toBe('The Beatles');
    expect(result.items[1]?.artist).toBe('Pink Floyd');
    expect(result.items[2]?.artist).toBe('Miles Davis');
  });

  it('trims whitespace from fields', () => {
    const csv = '  The Beatles  ,  Abbey Road  ,  1969  ';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.items[0]).toEqual({
      artist: 'The Beatles',
      title: 'Abbey Road',
      year: 1969,
    });
  });

  it('ignores empty lines', () => {
    const csv = `The Beatles,Abbey Road,1969

Pink Floyd,The Wall,1979

`;
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(2);
  });

  it('handles optional fields being empty', () => {
    const csv = 'The Beatles,Abbey Road,,,';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(0);
    expect(result.items[0]).toEqual({
      artist: 'The Beatles',
      title: 'Abbey Road',
    });
  });

  it('reports error for missing artist', () => {
    const csv = ',Abbey Road,1969';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toContain('Artist is required');
    expect(result.items).toHaveLength(0);
  });

  it('reports error for missing title', () => {
    const csv = 'The Beatles,,1969';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toContain('Title is required');
    expect(result.items).toHaveLength(0);
  });

  it('reports error for invalid year', () => {
    const csv = 'The Beatles,Abbey Road,not-a-year';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toContain('Invalid year');
    expect(result.items).toHaveLength(0);
  });

  it('reports error for missing required fields', () => {
    const csv = 'The Beatles';
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toContain('Missing required fields');
    expect(result.items).toHaveLength(0);
  });

  it('handles mixed valid and invalid lines', () => {
    const csv = `The Beatles,Abbey Road,1969
,Missing Artist,1970
Pink Floyd,The Wall,not-a-year
Miles Davis,Kind of Blue,1959`;
    const result = parseCSV(csv);

    expect(result.errors).toHaveLength(2);
    expect(result.items).toHaveLength(2);
    expect(result.items[0]?.artist).toBe('The Beatles');
    expect(result.items[1]?.artist).toBe('Miles Davis');
  });

  it('includes line numbers in error messages', () => {
    const csv = `The Beatles,Abbey Road,1969
,Missing Artist,1970
Pink Floyd,The Wall,1979`;
    const result = parseCSV(csv);

    expect(result.errors[0]).toContain('Line 2');
  });
});
