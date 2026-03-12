import { useState, useRef, useEffect } from "react";
import { fmtDate, daysInMonth, toDateStr } from "../../utils/format";
import "./DateRangePicker.css";

/** Date range picker with calendar - select FROM and TO by clicking */
export function DateRangePicker({
  dateFrom,
  dateTo,
  onChange,
}: {
  dateFrom: string;
  dateTo: string;
  onChange: (from: string, to: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [viewYear, setViewMonth_year] = useState(() => {
    const d = dateFrom ? new Date(dateFrom) : new Date();
    return d.getFullYear();
  });
  const [viewMonth, setViewMonth_month] = useState(() => {
    const d = dateFrom ? new Date(dateFrom) : new Date();
    return d.getMonth();
  });
  // Selection state: null = picking start, string = start picked, picking end
  const [pickStart, setPickStart] = useState<string | null>(null);
  const [hoverDate, setHoverDate] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setPickStart(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const prevMonth = () => {
    if (viewMonth === 0) { setViewMonth_year(viewYear - 1); setViewMonth_month(11); }
    else setViewMonth_month(viewMonth - 1);
  };
  const nextMonth = () => {
    if (viewMonth === 11) { setViewMonth_year(viewYear + 1); setViewMonth_month(0); }
    else setViewMonth_month(viewMonth + 1);
  };

  const handleDayClick = (dateStr: string) => {
    if (pickStart === null) {
      // First click: set start
      setPickStart(dateStr);
    } else {
      // Second click: set range (auto-swap if needed)
      const [a, b] = pickStart <= dateStr ? [pickStart, dateStr] : [dateStr, pickStart];
      onChange(a, b);
      setPickStart(null);
      setOpen(false);
    }
  };

  const today = new Date();
  const todayStr = toDateStr(today.getFullYear(), today.getMonth(), today.getDate());

  // Build calendar grid
  const firstDow = new Date(viewYear, viewMonth, 1).getDay(); // 0=Sun
  const totalDays = daysInMonth(viewYear, viewMonth);
  const weeks: (number | null)[][] = [];
  let week: (number | null)[] = Array(firstDow).fill(null);
  for (let d = 1; d <= totalDays; d++) {
    week.push(d);
    if (week.length === 7) { weeks.push(week); week = []; }
  }
  if (week.length > 0) {
    while (week.length < 7) week.push(null);
    weeks.push(week);
  }

  // Determine effective range for highlighting
  const effFrom = pickStart ?? dateFrom;
  const effTo = pickStart ? (hoverDate ?? pickStart) : dateTo;
  const rangeStart = effFrom <= effTo ? effFrom : effTo;
  const rangeEnd = effFrom <= effTo ? effTo : effFrom;

  return (
    <div className="drp-container" ref={containerRef}>
      <button className="drp-trigger" onClick={() => { setOpen(!open); setPickStart(null); }}>
        {dateFrom && dateTo ? `${fmtDate(dateFrom)} 〜 ${fmtDate(dateTo)}` : "全期間"}
      </button>
      {open && (
        <div className="drp-dropdown">
          <div className="drp-calendar">
            <div className="drp-nav">
              <button onClick={prevMonth}>&lt;</button>
              <span>{viewYear}年{viewMonth + 1}月</span>
              <button onClick={nextMonth}>&gt;</button>
            </div>
            <div className="drp-grid">
              {["日", "月", "火", "水", "木", "金", "土"].map((w) => (
                <div key={w} className="drp-dow">{w}</div>
              ))}
              {weeks.flat().map((day, i) => {
                if (day === null) return <div key={`e${i}`} className="drp-cell drp-empty" />;
                const ds = toDateStr(viewYear, viewMonth, day);
                const isInRange = ds >= rangeStart && ds <= rangeEnd;
                const isStart = ds === rangeStart;
                const isEnd = ds === rangeEnd;
                const isToday = ds === todayStr;
                return (
                  <div
                    key={ds}
                    className={[
                      "drp-cell",
                      isInRange ? "drp-in-range" : "",
                      isStart ? "drp-start" : "",
                      isEnd ? "drp-end" : "",
                      isToday ? "drp-today" : "",
                    ].join(" ")}
                    onClick={() => handleDayClick(ds)}
                    onMouseEnter={() => setHoverDate(ds)}
                    onMouseLeave={() => setHoverDate(null)}
                  >
                    {day}
                  </div>
                );
              })}
            </div>
          </div>
          {pickStart && (
            <div className="drp-hint">終了日を選択してください</div>
          )}
        </div>
      )}
    </div>
  );
}
