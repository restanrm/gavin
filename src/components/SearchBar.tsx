interface SearchBarProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export function SearchBar({ value, onChange, placeholder = 'Search by artist or title...' }: SearchBarProps) {
  return (
    <div className="search-bar">
      <label htmlFor="vinyl-search" className="sr-only">
        Search vinyls
      </label>
      <input
        id="vinyl-search"
        type="search"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="search-input"
        aria-label="Search vinyls by artist or title"
      />
    </div>
  );
}
