#!/usr/bin/python3

from cgi import test
import re
import sys

def read_test_data(filename):
    prev_pattern = None
    for line in open(filename, encoding="utf8"):
        if line[0] == '#':
            continue

        fields = [field.strip() for field in line.split("\t")][:4]
        if len(fields) < 4:
            continue

        [flags, pattern, string, output, *_] = fields
        if pattern == "SAME":
            pattern = prev_pattern
        prev_pattern = pattern
        if "E" not in flags:
            continue
        submatches = []
        if output != "NOMATCH":
            for match in re.finditer("\(([^,]*),([^)]*)\)", output):
                if match[1] == '?' and match[2] == '?':
                    submatches.append(None)
                else:
                    submatches.append([int(match[1]), int(match[2])])
        yield [pattern, string, submatches]


def main():
    filename = sys.argv[1]

    tests = list(read_test_data(filename))

    print("//! Generated by:")
    print("//! generate_fowler_test_data.py <filename>")
    print()
    print("static TEST_DATA: [(&'static str, &'static str, &'static [Option<Range<usize>]); %d] = [" % len(tests))
    for [pattern, string, submatches] in tests:
        print("    (r#\"%s\"#, \"%s\")," % (pattern, string))
    print("];")

if __name__ == '__main__':
    main()