export function parseGenreInput(value: string): string[] | null {
  const genres = value
    .split(',')
    .map((genre) => genre.trim())
    .filter((genre) => genre.length > 0)
    .reduce<string[]>((uniqueGenres, genre) => {
      if (!uniqueGenres.some((existing) => existing.toLowerCase() === genre.toLowerCase())) {
        uniqueGenres.push(genre);
      }
      return uniqueGenres;
    }, []);

  return genres.length > 0 ? genres : null;
}

export function formatGenres(genres?: string[] | null): string {
  return genres?.join(', ') ?? '';
}

export function hasGenres(genres?: string[] | null): boolean {
  return Boolean(genres?.some((genre) => genre.trim().length > 0));
}
