#!/usr/bin/env python
"""Small XLSX exploration helpers (inspect/preview/export)."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, List, Optional, Sequence, Tuple

import xlsx2csv


REPORTS_DIR = Path(__file__).resolve().parent.parent / "data" / "import_reports"


class StopParsing(Exception):
    pass


@dataclass
class SheetSummary:
    name: str
    rows: int
    cols: int
    header_row: Optional[int]
    header: List[str]


@dataclass
class InspectReport:
    file_name: str
    file_path: str
    file_sha256: str
    generated_at_utc: str
    workbook_name: str
    sheets: List[SheetSummary]


NUMERIC_RE = re.compile(r"^-?\d+(\.\d+)?([eE]-?\d+)?$")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def now_utc_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def normalize_header_cell(value: str, fallback: str) -> str:
    cleaned = " ".join(value.strip().split())
    return cleaned if cleaned else fallback


def dedupe_headers(headers: Sequence[str]) -> List[str]:
    seen = {}
    deduped = []
    for h in headers:
        key = h
        count = seen.get(key, 0) + 1
        seen[key] = count
        if count == 1:
            deduped.append(h)
        else:
            deduped.append(f"{h}_{count}")
    return deduped


def is_numeric(value: str) -> bool:
    return bool(NUMERIC_RE.match(value.strip()))


def guess_header(rows: Sequence[Sequence[str]], max_scan: int = 50) -> Tuple[Optional[int], List[str]]:
    for idx, row in enumerate(rows[:max_scan], start=1):
        non_empty = [c for c in row if c and c.strip()]
        if len(non_empty) < 2:
            continue
        non_numeric = [c for c in non_empty if not is_numeric(c)]
        if len(non_numeric) / len(non_empty) < 0.6:
            continue
        headers = []
        for i, cell in enumerate(row, start=1):
            headers.append(normalize_header_cell(cell, f"col_{i}"))
        return idx, dedupe_headers(headers)
    return None, []


class RowCollector:
    def __init__(self, collect_rows: int = 0, stop_after: Optional[int] = None):
        self.collect_rows = collect_rows
        self.stop_after = stop_after
        self.rows: List[List[str]] = []
        self.row_count = 0
        self.max_cols = 0

    def writerow(self, row: Sequence[str]) -> None:
        self.row_count += 1
        self.max_cols = max(self.max_cols, len(row))
        if self.collect_rows and len(self.rows) < self.collect_rows:
            self.rows.append(list(row))
        if self.stop_after and self.row_count >= self.stop_after:
            raise StopParsing()


def open_sheet(x2c: xlsx2csv.Xlsx2csv, sheet_index: int) -> Tuple[xlsx2csv.Sheet, object, str]:
    sheets_filtered = [s for s in x2c.workbook.sheets if s["index"] == sheet_index]
    if not sheets_filtered:
        raise xlsx2csv.XlsxValueError(f"Sheet with index {sheet_index} not found")

    sheet_path = None
    sheet = sheets_filtered[0]
    relation_id = sheet.get("relation_id")
    if relation_id:
        relationship = x2c.workbook.relationships.relationships.get(relation_id)
        if relationship and "target" in relationship:
            sheet_path = relationship["target"]
            if not (sheet_path.startswith("/xl/") or sheet_path.startswith("xl/")):
                sheet_path = "/xl/" + sheet_path

    sheet_file = None
    if sheet_path is None:
        sheet_path = f"/xl/worksheets/sheet{sheet_index}.xml"
        sheet_file = x2c._filehandle(sheet_path)
        if sheet_file is None:
            sheet_path = None
    if sheet_path is None:
        sheet_path = f"/xl/worksheets/worksheet{sheet_index}.xml"
        sheet_file = x2c._filehandle(sheet_path)
        if sheet_file is None:
            sheet_path = None
    if sheet_path is None and sheet_index == 1:
        sheet_path = x2c.content_types.types["worksheet"]
        sheet_file = x2c._filehandle(sheet_path)
        if sheet_file is None:
            sheet_path = None
    if sheet_file is None and sheet_path is not None:
        sheet_file = x2c._filehandle(sheet_path)
    if sheet_file is None:
        raise xlsx2csv.SheetNotFoundException(f"Sheet {sheet_index} not found")

    sheet_obj = xlsx2csv.Sheet(x2c.workbook, x2c.shared_strings, x2c.styles, sheet_file)
    relationships_path = os.path.join(
        os.path.dirname(sheet_path),
        "_rels",
        os.path.basename(sheet_path) + ".rels",
    )
    sheet_obj.relationships = x2c._parse(xlsx2csv.Relationships, relationships_path)
    sheet_obj.set_dateformat(x2c.options["dateformat"])
    sheet_obj.set_timeformat(x2c.options["timeformat"])
    sheet_obj.set_floatformat(x2c.options["floatformat"])
    sheet_obj.set_skip_empty_lines(x2c.options["skip_empty_lines"])
    sheet_obj.set_skip_trailing_columns(x2c.options["skip_trailing_columns"])
    sheet_obj.set_include_hyperlinks(x2c.options["hyperlinks"])
    sheet_obj.set_merge_cells(x2c.options["merge_cells"])
    sheet_obj.set_scifloat(x2c.options["scifloat"])
    sheet_obj.set_ignore_formats(x2c.options["ignore_formats"])
    sheet_obj.set_skip_hidden_rows(x2c.options["skip_hidden_rows"])
    sheet_obj.set_no_line_breaks(x2c.options["no_line_breaks"])

    if x2c.options["escape_strings"] and sheet_obj.filedata:
        sheet_obj.filedata = re.sub(
            r"(<v>[^<>]+)&#10;([^<>]+</v>)",
            r"\1\\n\2",
            re.sub(
                r"(<v>[^<>]+)&#9;([^<>]+</v>)",
                r"\1\\t\2",
                re.sub(r"(<v>[^<>]+)&#13;([^<>]+</v>)", r"\1\\r\2", sheet_obj.filedata.decode()),
            ),
        )

    return sheet_obj, sheet_file, sheet_path


def build_xlsx_reader(path: Path) -> xlsx2csv.Xlsx2csv:
    return xlsx2csv.Xlsx2csv(
        str(path),
        delimiter=",",
        quoting=csv.QUOTE_MINIMAL,
        lineterminator="\n",
        skip_empty_lines=False,
        skip_trailing_columns=False,
        dateformat="float",
        floatformat=None,
        scifloat=True,
        ignore_formats=["date", "time", "float"],
    )


def iter_sheet_rows(
    x2c: xlsx2csv.Xlsx2csv,
    sheet_name: str,
    collector: RowCollector,
) -> None:
    sheet_index = x2c.getSheetIdByName(sheet_name)
    if not sheet_index:
        raise xlsx2csv.SheetNotFoundException(f"Sheet '{sheet_name}' not found")
    sheet, sheet_file, _ = open_sheet(x2c, sheet_index)
    try:
        sheet.to_csv(collector)
    except StopParsing:
        pass
    finally:
        sheet_file.close()
        sheet.close()


def format_tsv_row(row: Sequence[str]) -> str:
    return "\t".join("" if cell is None else str(cell) for cell in row)


def parse_columns_filter(value: Optional[str]) -> Tuple[str, Optional[object]]:
    if not value:
        return "all", None
    if value.startswith("/") and value.endswith("/") and len(value) > 2:
        return "regex", re.compile(value[1:-1])
    names = [normalize_header_cell(v, v) for v in value.split(",") if v.strip()]
    return "list", {n.lower() for n in names}


def select_column_indices(headers: Sequence[str], filt: Tuple[str, Optional[object]]) -> List[int]:
    mode, payload = filt
    if mode == "all":
        return list(range(len(headers)))
    if mode == "regex":
        regex = payload
        return [i for i, h in enumerate(headers) if regex.search(h)]
    names = payload
    return [i for i, h in enumerate(headers) if h.lower() in names]


def inspect_workbook(file_path: Path) -> InspectReport:
    reader = build_xlsx_reader(file_path)
    sheets = []
    for sheet in reader.workbook.sheets:
        name = sheet["name"]
        collector = RowCollector(collect_rows=50)
        iter_sheet_rows(reader, name, collector)
        header_row, headers = guess_header(collector.rows)
        sheets.append(
            SheetSummary(
                name=name,
                rows=collector.row_count,
                cols=collector.max_cols,
                header_row=header_row,
                header=headers,
            )
        )

    return InspectReport(
        file_name=file_path.name,
        file_path=str(file_path.resolve()),
        file_sha256=sha256_file(file_path),
        generated_at_utc=now_utc_iso(),
        workbook_name=file_path.stem,
        sheets=sheets,
    )


def write_report(report: InspectReport) -> Path:
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)
    report_path = REPORTS_DIR / f"{report.file_sha256}.json"
    payload = {
        "file_name": report.file_name,
        "file_path": report.file_path,
        "file_sha256": report.file_sha256,
        "generated_at_utc": report.generated_at_utc,
        "workbook_name": report.workbook_name,
        "sheets": [
            {
                "name": s.name,
                "rows": s.rows,
                "cols": s.cols,
                "header_row": s.header_row,
                "header": s.header,
            }
            for s in report.sheets
        ],
    }
    report_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    return report_path


def cmd_inspect(args: argparse.Namespace) -> int:
    file_path = Path(args.file)
    report = inspect_workbook(file_path)
    report_path = write_report(report)

    print(f"Workbook: {report.workbook_name}")
    print(f"File: {report.file_name}")
    print(f"SHA256: {report.file_sha256}")
    print(f"Sheets: {len(report.sheets)}")
    for sheet in report.sheets:
        header_preview = ", ".join(sheet.header[:10]) if sheet.header else "(no header guess)"
        header_row = sheet.header_row if sheet.header_row else "?"
        print(
            f"- {sheet.name}: rows~{sheet.rows} cols~{sheet.cols} "
            f"header_row={header_row} header=[{header_preview}]"
        )
    print(f"Report: {report_path}")
    return 0


def cmd_preview(args: argparse.Namespace) -> int:
    file_path = Path(args.file)
    reader = build_xlsx_reader(file_path)
    collector = RowCollector(collect_rows=args.rows, stop_after=args.rows)
    iter_sheet_rows(reader, args.sheet, collector)
    for row in collector.rows:
        print(format_tsv_row(row))
    return 0


def cmd_export(args: argparse.Namespace) -> int:
    file_path = Path(args.file)
    reader = build_xlsx_reader(file_path)

    header_scan = RowCollector(collect_rows=50, stop_after=50)
    iter_sheet_rows(reader, args.sheet, header_scan)
    header_row_idx, headers = guess_header(header_scan.rows)
    if header_row_idx is None:
        header_row_idx = 1
        if header_scan.rows:
            headers = [normalize_header_cell(v, f"col_{i+1}") for i, v in enumerate(header_scan.rows[0])]
            headers = dedupe_headers(headers)
        else:
            raise SystemExit("No rows found in sheet; cannot export")

    filt = parse_columns_filter(args.columns)
    selected = select_column_indices(headers, filt)
    if not selected:
        raise SystemExit("No columns matched the filter")

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, quoting=csv.QUOTE_MINIMAL, lineterminator="\n")
        writer.writerow([headers[i] for i in selected])

        row_counter = {"index": 0}

        def on_row(row: Sequence[str], row_index: int) -> None:
            if row_index < header_row_idx + 1:
                return
            values = []
            for i in selected:
                raw = row[i] if i < len(row) else ""
                if raw is None:
                    raw = ""
                cell = str(raw)
                if not cell.strip():
                    values.append("NULL")
                else:
                    values.append(cell)
            writer.writerow(values)

        collector = RowCollector()

        def writerow(row: Sequence[str]) -> None:
            row_counter["index"] += 1
            on_row(row, row_counter["index"])

        collector.writerow = writerow  # type: ignore[assignment]
        iter_sheet_rows(reader, args.sheet, collector)

    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="XLSX exploration helpers")
    sub = parser.add_subparsers(dest="command", required=True)

    inspect_parser = sub.add_parser("inspect", help="Inspect workbook and write report")
    inspect_parser.add_argument("--file", required=True, help="Path to .xlsx file")
    inspect_parser.set_defaults(func=cmd_inspect)

    preview_parser = sub.add_parser("preview", help="Preview sheet as TSV")
    preview_parser.add_argument("--file", required=True, help="Path to .xlsx file")
    preview_parser.add_argument("--sheet", required=True, help="Sheet name")
    preview_parser.add_argument("--rows", type=int, default=30, help="Rows to preview")
    preview_parser.set_defaults(func=cmd_preview)

    export_parser = sub.add_parser("export", help="Export sheet to CSV")
    export_parser.add_argument("--file", required=True, help="Path to .xlsx file")
    export_parser.add_argument("--sheet", required=True, help="Sheet name")
    export_parser.add_argument("--out", required=True, help="Output CSV path")
    export_parser.add_argument("--columns", default=None, help="Regex (/.../) or comma list")
    export_parser.set_defaults(func=cmd_export)

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
