# Parallelism

CPython has the infamous GIL (Global Interpreter Lock), which prevents developers
from getting true parallelism when running pure Python code. While PyO3 needs to
hold the GIL by default when called from Python, in order to allow manipulation
of Python objects, you can release the GIL when executing Rust-only code to
achieve true parallelism.

The [`Python::allow_threads`] method temporarily releases the GIL, thus allowing other Python threads to run.

```rust,ignore
impl Python {
    pub fn allow_threads<T, F>(self, f: F) -> T where F: Send + FnOnce() -> T {}
}
```

Let's take a look at our [word-count](https://github.com/PyO3/pyo3/blob/master/examples/word-count/src/lib.rs) example,
where we have a `wc_parallel` function that utilizes the [rayon](https://github.com/nikomatsakis/rayon) crate to count words in parallel.

```rust,ignore
fn wc_parallel(lines: &str, search: &str) -> i32 {
    lines.par_lines()
         .map(|line| wc_line(line, search))
         .sum()
}
```

Then in the Python bridge, we have a function `search` exposed to the Python runtime which calls
`wc_parallel` inside a closure passed to [`Python::allow_threads`] to enable true parallelism:

```rust,ignore
#[pymodule]
fn word_count(py: Python, m: &PyModule) -> PyResult<()> {

    #[pyfn(m, "search")]
    fn search(py: Python, path: String, search: String) -> PyResult<i32> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let count = py.allow_threads(move || wc_parallel(&contents, &search));
        Ok(count)
    }

    Ok(())
}
```

## Benchmark

Let's benchmark the `word-count` example to verify that we did unlock true parallelism with PyO3.
We are using `pytest-benchmark` to benchmark three word count functions:

1. [Pure Python version](https://github.com/PyO3/pyo3/blob/master/examples/word-count/word_count/__init__.py#L9)
2. [Rust sequential version](https://github.com/PyO3/pyo3/blob/master/examples/word-count/src/lib.rs#L64)
3. [Rust parallel version](https://github.com/PyO3/pyo3/blob/master/examples/word-count/src/lib.rs#L54)

The benchmark script can be found [here](https://github.com/PyO3/pyo3/blob/master/examples/word-count/tests/test_word_count.py),
then we can run `pytest tests` to benchmark them.

On MacBook Pro (Retina, 15-inch, Mid 2015) the benchmark gives:

![Benchmark Result](https://user-images.githubusercontent.com/1556054/28604608-81bd6d22-71fe-11e7-8a2c-c3cf3bd0f622.png)


[`Python::allow_threads`]: https://docs.rs/pyo3/latest/pyo3/struct.Python.html#method.allow_threads
