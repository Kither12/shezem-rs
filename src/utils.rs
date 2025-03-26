use std::cmp::Ordering::{Greater, Less};

pub fn longest_increasing_subsequence<T>(a: &[T]) -> Vec<T>
where
    T: Ord + Clone,
{
    if a.is_empty() {
        return Vec::new();
    }

    let mut m: Vec<usize> = Vec::with_capacity(a.len());
    let mut p: Vec<usize> = vec![0; a.len()];

    for i in 0..a.len() {
        match m.binary_search_by(|&j| if a[j] > a[i] { Greater } else { Less }) {
            Ok(pos) => {
                m[pos] = i;
                if pos > 0 {
                    p[i] = m[pos - 1];
                }
            }
            Err(pos) => {
                if pos > 0 {
                    p[i] = m[pos - 1];
                }
                if pos == m.len() {
                    m.push(i);
                } else {
                    m[pos] = i;
                }
            }
        }
    }

    let mut result = Vec::with_capacity(m.len());
    if !m.is_empty() {
        let mut k = m[m.len() - 1];
        result.push(a[k].clone());

        for _ in 0..m.len() - 1 {
            k = p[k];
            result.push(a[k].clone());
        }

        result.reverse();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let empty: Vec<i32> = vec![];
        assert_eq!(longest_increasing_subsequence(&empty), vec![]);
    }

    #[test]
    fn test_single_element() {
        let single = vec![5];
        assert_eq!(longest_increasing_subsequence(&single), vec![5]);
    }

    #[test]
    fn test_all_same() {
        let all_same = vec![3, 3, 3, 3];
        assert_eq!(longest_increasing_subsequence(&all_same), vec![3, 3, 3, 3]);
    }

    #[test]
    fn test_strictly_increasing() {
        let increasing = vec![1, 2, 3, 4, 5];
        assert_eq!(
            longest_increasing_subsequence(&increasing),
            vec![1, 2, 3, 4, 5]
        );
    }
}
