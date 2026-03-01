import { useState, useEffect } from 'react';

interface Row {
  from: string;
  to: string;
}

interface DictionaryEditorProps {
  corrections: Record<string, string>;
  onChange: (corrections: Record<string, string>) => void;
}

function recordToRows(record: Record<string, string>): Row[] {
  return Object.entries(record).map(([from, to]) => ({ from, to }));
}

function rowsToRecord(rows: Row[]): Record<string, string> {
  const record: Record<string, string> = {};
  for (const row of rows) {
    const key = row.from.trim();
    if (key) {
      record[key] = row.to;
    }
  }
  return record;
}

export function DictionaryEditor({ corrections, onChange }: DictionaryEditorProps) {
  const [rows, setRows] = useState<Row[]>(() => recordToRows(corrections));

  // Sync rows when corrections prop changes (e.g. profile switch)
  useEffect(() => {
    setRows(recordToRows(corrections));
  }, [corrections]);

  function handleFromChange(index: number, value: string) {
    const next = rows.map((row, i) => (i === index ? { ...row, from: value } : row));
    setRows(next);
  }

  function handleToChange(index: number, value: string) {
    const next = rows.map((row, i) => (i === index ? { ...row, to: value } : row));
    setRows(next);
  }

  function handleBlur() {
    onChange(rowsToRecord(rows));
  }

  function handleDelete(index: number) {
    const next = rows.filter((_, i) => i !== index);
    setRows(next);
    onChange(rowsToRecord(next));
  }

  function handleAdd() {
    setRows([...rows, { from: '', to: '' }]);
  }

  const inputClass =
    'w-full rounded border border-gray-300 px-2 py-1 text-sm focus:outline-none focus:border-indigo-400 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100';

  return (
    <div>
      {rows.length === 0 ? (
        <p className="text-sm text-gray-400 dark:text-gray-500">
          No corrections. Click + Add entry to create one.
        </p>
      ) : (
        <table className="w-full table-fixed border-collapse">
          <thead>
            <tr>
              <th className="pb-1.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400 w-[45%]">
                From
              </th>
              <th className="pb-1.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400 w-[45%]">
                To
              </th>
              <th className="pb-1.5 w-[10%]" />
            </tr>
          </thead>
          <tbody>
            {rows.map((row, i) => (
              <tr key={i} className="group">
                <td className="pr-2 py-1">
                  <input
                    type="text"
                    value={row.from}
                    onChange={(e) => handleFromChange(i, e.target.value)}
                    onBlur={handleBlur}
                    placeholder="why section"
                    className={inputClass}
                  />
                </td>
                <td className="pr-2 py-1">
                  <input
                    type="text"
                    value={row.to}
                    onChange={(e) => handleToChange(i, e.target.value)}
                    onBlur={handleBlur}
                    placeholder="W-section"
                    className={inputClass}
                  />
                </td>
                <td className="py-1 text-center">
                  <button
                    onClick={() => handleDelete(i)}
                    className="text-gray-400 transition-colors hover:text-red-500 focus:outline-none"
                    title="Delete entry"
                  >
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <button
        onClick={handleAdd}
        className="mt-3 text-sm text-indigo-600 hover:text-indigo-800 dark:text-indigo-400 dark:hover:text-indigo-300 focus:outline-none"
      >
        + Add entry
      </button>

      <p className="mt-2 text-xs text-gray-400 dark:text-gray-500">
        Matches whole words only. Case-insensitive.
      </p>
    </div>
  );
}
