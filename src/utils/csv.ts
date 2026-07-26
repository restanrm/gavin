import type { BulkImportItem } from '../types';
import { parseGenreInput } from './genres';

export interface ParseResult {
  items: BulkImportItem[];
  errors: string[];
}

/**
 * Parse CSV text into bulk import items
 * Format: artist,title,year,notes,cover_url,genre
 * - artist and title are required
 * - year must be a valid number if present
 * - notes, cover_url, and genre are optional
 */
export function parseCSV(text: string): ParseResult {
  const items: BulkImportItem[] = [];
  const errors: string[] = [];

  const lines = text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  lines.forEach((line, index) => {
    const lineNumber = index + 1;
    const parts = line.split(',').map((part) => part.trim());

    if (parts.length < 2) {
      errors.push(`Line ${lineNumber}: Missing required fields (artist, title)`);
      return;
    }

    const [artist, title, yearStr, notes, coverUrl, genre] = parts;

    if (!artist) {
      errors.push(`Line ${lineNumber}: Artist is required`);
      return;
    }

    if (!title) {
      errors.push(`Line ${lineNumber}: Title is required`);
      return;
    }

    const item: BulkImportItem = {
      artist,
      title,
    };

    // Parse optional year
    if (yearStr && yearStr.length > 0) {
      const year = parseInt(yearStr, 10);
      if (isNaN(year)) {
        errors.push(`Line ${lineNumber}: Invalid year "${yearStr}"`);
        return;
      }
      item.year = year;
    }

    // Add optional notes
    if (notes && notes.length > 0) {
      item.notes = notes;
    }

    // Add optional cover URL
    if (coverUrl && coverUrl.length > 0) {
      item.cover_url = coverUrl;
    }

    // Add optional genre
    if (genre && genre.length > 0) {
      item.genre = parseGenreInput(genre) ?? undefined;
    }

    items.push(item);
  });

  return { items, errors };
}
