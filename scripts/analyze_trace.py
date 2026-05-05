#!/usr/bin/env python3
import re
import argparse
from dataclasses import dataclass
from typing import Dict, Tuple, Optional


@dataclass
class MethodStats:
    class_name: str
    method_name: str
    call_count: int = 0
    total_time_us: float = 0.0
    self_time_us: float = 0.0
    min_time_us: float = float('inf')
    max_time_us: float = 0.0

    def record(self, duration_us: float, self_duration_us: float):
        self.call_count += 1
        self.total_time_us += duration_us
        self.self_time_us += self_duration_us
        if duration_us < self.min_time_us:
            self.min_time_us = duration_us
        if duration_us > self.max_time_us:
            self.max_time_us = duration_us

    @property
    def avg_time_us(self) -> float:
        if self.call_count == 0:
            return 0.0
        return self.total_time_us / self.call_count


@dataclass 
class StackFrame:
    class_name: str
    method_name: str
    child_time_us: float = 0.0


def parse_duration(duration_str: str) -> Optional[float]:
    """Parse duration string like '2.09ms', '123us', '1.50s' to microseconds."""
    match = re.match(r'([\d.]+)(us|ms|s)', duration_str)
    if not match:
        return None
    value = float(match.group(1))
    unit = match.group(2)
    if unit == 'us':
        return value
    elif unit == 'ms':
        return value * 1000
    elif unit == 's':
        return value * 1_000_000
    return None


def parse_trace_file(filepath: str) -> Dict[str, Dict[Tuple[str, str], MethodStats]]:
    """Parse trace file and collect method statistics per thread with self-time calculation."""
    stats: Dict[str, Dict[Tuple[str, str], MethodStats]] = {}  # thread_id -> method stats
    stacks: Dict[str, list] = {}  # thread_id -> list of StackFrame

    entry_pattern = re.compile(
        r'\[[\d:.]+\] >> \[([^\]]+)\]\s+[N ]?\s*'
        r'(L[^;]+;)\s*->\s*([^\s]+)'
    )

    exit_pattern = re.compile(
        r'\[[\d:.]+\] << \[([^\]]+)\]\s+[N ]?\s*'
        r'(L[^;]+;)\s*->\s*([^\s]+)'
        r'(?:\s+([\d.]+(?:us|ms|s)))?'
    )

    with open(filepath, 'r') as f:
        for line in f:
            entry_match = entry_pattern.search(line)
            if entry_match:
                thread_id = entry_match.group(1)
                class_name = entry_match.group(2)
                method_name = entry_match.group(3)

                if thread_id not in stacks:
                    stacks[thread_id] = []
                stacks[thread_id].append(StackFrame(class_name, method_name))
                continue

            exit_match = exit_pattern.search(line)
            if exit_match:
                thread_id = exit_match.group(1)
                class_name = exit_match.group(2)
                method_name = exit_match.group(3)
                duration_str = exit_match.group(4)

                if not duration_str:
                    continue

                duration_us = parse_duration(duration_str)
                if duration_us is None:
                    continue

                stack = stacks.get(thread_id, [])

                child_time_us = 0.0
                for i in range(len(stack) - 1, -1, -1):
                    if stack[i].class_name == class_name and stack[i].method_name == method_name:
                        child_time_us = stack[i].child_time_us
                        stack.pop(i)
                        break

                self_time_us = max(0, duration_us - child_time_us)

                if stack:
                    stack[-1].child_time_us += duration_us

                if thread_id not in stats:
                    stats[thread_id] = {}
                thread_stats = stats[thread_id]

                key = (class_name, method_name)
                if key not in thread_stats:
                    thread_stats[key] = MethodStats(class_name, method_name)
                thread_stats[key].record(duration_us, self_time_us)

    return stats


def format_duration(us: float) -> str:
    """Format duration in human-readable form."""
    if us < 1000:
        return f"{us:.0f}us"
    elif us < 1_000_000:
        return f"{us/1000:.2f}ms"
    else:
        return f"{us/1_000_000:.2f}s"


def print_thread_table(thread_id: str, stats: Dict[Tuple[str, str], MethodStats], sort_by: str = 'self'):
    """Print summary table for a single thread."""
    stats_list = list(stats.values())

    if sort_by == 'self':
        stats_list.sort(key=lambda s: s.self_time_us, reverse=True)
    elif sort_by == 'total':
        stats_list.sort(key=lambda s: s.total_time_us, reverse=True)
    elif sort_by == 'avg':
        stats_list.sort(key=lambda s: s.avg_time_us, reverse=True)
    elif sort_by == 'calls':
        stats_list.sort(key=lambda s: s.call_count, reverse=True)
    elif sort_by == 'max':
        stats_list.sort(key=lambda s: s.max_time_us, reverse=True)

    total_events = sum(s.call_count for s in stats_list)
    self_total = sum(s.self_time_us for s in stats_list)

    print()
    print("=" * 120)
    print(f" Thread: {thread_id} ")
    print("=" * 120)
    print(f"Method exits: {total_events} | Unique methods: {len(stats_list)} | Total self time: {self_total/1000:.2f}ms")
    print()

    print(f"{'Self%':>7} {'Calls':>10} {'Self(ms)':>12} {'Total(ms)':>12} {'Avg(us)':>12} {'Max(us)':>12}  Method")
    print("-" * 120)

    for s in stats_list:
        self_ms = s.self_time_us / 1000
        total_ms = s.total_time_us / 1000
        self_pct = (s.self_time_us / self_total * 100) if self_total > 0 else 0
        print(f"{self_pct:>6.1f}% {s.call_count:>10} {self_ms:>12.2f} {total_ms:>12.2f} {s.avg_time_us:>12.0f} {s.max_time_us:>12.0f}  {s.class_name} -> {s.method_name}")


def print_summary(all_stats: Dict[str, Dict[Tuple[str, str], MethodStats]], sort_by: str = 'self'):
    """Print summary tables, one per thread."""
    if not all_stats:
        print("No method timing data found.")
        return

    total_threads = len(all_stats)
    total_methods = sum(len(s) for s in all_stats.values())
    total_events = sum(
        sum(m.call_count for m in thread_stats.values())
        for thread_stats in all_stats.values()
    )

    print()
    print("=" * 120)
    print(" Overall Summary ")
    print("=" * 120)
    print(f"Threads: {total_threads} | Total method exits: {total_events} | Unique methods across all threads: {total_methods}")

    sorted_threads = sorted(
        all_stats.items(),
        key=lambda x: sum(m.self_time_us for m in x[1].values()),
        reverse=True
    )

    for thread_id, thread_stats in sorted_threads:
        print_thread_table(thread_id, thread_stats, sort_by)

    print()
    print("Self%: time in function only (excludes callees) - like flamegraph")
    print("Total: time including all nested calls")
    print("=" * 120)


def load_ignore_patterns(filepath: str) -> list:
    """Load ignore patterns from file. One pattern per line."""
    patterns = []
    with open(filepath, 'r') as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#'):
                parts = line.split('->')
                if len(parts) == 2:
                    class_pat = parts[0].strip()
                    method_pat = parts[1].strip()
                    patterns.append((class_pat, method_pat))
                else:
                    patterns.append((line, None))
    return patterns


def matches_ignore(class_name: str, method_name: str, patterns: list) -> bool:
    """Check if a method matches any ignore pattern."""
    full_name = f"{class_name} -> {method_name}"
    for class_pat, method_pat in patterns:
        if method_pat:
            if class_pat in class_name and method_pat in method_name:
                return True
        else:
            if class_pat in full_name:
                return True
    return False


def main():
    parser = argparse.ArgumentParser(description='Analyze apktrace trace files')
    parser.add_argument('trace_file', help='Path to trace file')
    parser.add_argument('--sort-by', choices=['self', 'total', 'avg', 'calls', 'max'],
                       default='self', help='Sort by field (default: self)')
    parser.add_argument('--filter', metavar='PATTERN', help='Include only methods matching pattern')
    parser.add_argument('--ignore', metavar='FILE', help='File with patterns to ignore (one per line)')
    parser.add_argument('--ignore-pattern', metavar='PATTERN', action='append',
                       help='Ignore methods matching pattern (can use multiple times)')

    args = parser.parse_args()

    print(f"Parsing {args.trace_file}...")
    all_stats = parse_trace_file(args.trace_file)

    if args.filter:
        pattern = re.compile(args.filter)
        for thread_id in all_stats:
            all_stats[thread_id] = {
                k: v for k, v in all_stats[thread_id].items()
                if pattern.search(k[0]) or pattern.search(k[1])
            }

    ignore_patterns = []
    if args.ignore:
        ignore_patterns.extend(load_ignore_patterns(args.ignore))
        print(f"Loaded {len(ignore_patterns)} ignore patterns from {args.ignore}")

    if args.ignore_pattern:
        for pat in args.ignore_pattern:
            parts = pat.split('->')
            if len(parts) == 2:
                ignore_patterns.append((parts[0].strip(), parts[1].strip()))
            else:
                ignore_patterns.append((pat, None))

    if ignore_patterns:
        total_before = sum(len(s) for s in all_stats.values())
        for thread_id in all_stats:
            all_stats[thread_id] = {
                k: v for k, v in all_stats[thread_id].items()
                if not matches_ignore(k[0], k[1], ignore_patterns)
            }
        total_after = sum(len(s) for s in all_stats.values())
        print(f"Filtered out {total_before - total_after} methods")

    all_stats = {tid: s for tid, s in all_stats.items() if s}

    print_summary(all_stats, sort_by=args.sort_by)


if __name__ == '__main__':
    main()
