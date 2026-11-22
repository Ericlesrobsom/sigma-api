use sigma_core::cap;

// Vetor real capturado em 2026-11-22 (token de CAP já expirado, sem valor
// de credencial). Serve para garantir que o porte do algoritmo não regride.
const TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJuIjoiNDIwYmFjMzY5ZTcwNzI0NmNhNGJhMGE4YjYzZjRlN2E0NTVmMmJjZGI2ZWU5YjEzYzQiLCJjIjo4MCwicyI6MzIsImQiOjQsImV4cCI6MTc5NTM4MjI0NjAwNiwiaWF0IjoxNzk1MzgxMzQ2MDA2LCJzayI6IjA0NTRkMTI2MmIifQ.HwstPKHVfEehJQLq0WcaRlO3Okv8wPedYUX15wp4eho";
const SOLUTIONS: [u64; 80] = [
    38864, 20368, 16447, 316179, 43063, 45635, 4527, 3950, 21098, 11676, 144757, 206034, 9244,
    23658, 122077, 127486, 40175, 196167, 37684, 57124, 3675, 12817, 10524, 69031, 51192, 21500,
    45360, 14198, 33578, 220568, 45853, 171287, 48096, 24395, 85992, 28457, 2023, 81594, 75055,
    71453, 22690, 12335, 10089, 89784, 56053, 22380, 39139, 53089, 31703, 43090, 19703, 4598,
    74996, 34119, 18738, 67915, 100426, 145437, 19852, 20660, 73455, 38778, 35251, 98670, 49188,
    4325, 66491, 21320, 37912, 6407, 35242, 58499, 38356, 86839, 126050, 21559, 27689, 304762,
    220978, 22490,
];

#[test]
fn vetor_real_do_painel_bate_80_de_80() {
    for (i, solution) in SOLUTIONS.iter().enumerate() {
        assert!(
            cap::verify_solution(TOKEN, i, 32, 4, *solution),
            "solucao {i} nao bateu"
        );
    }
}

#[test]
fn solver_resolve_um_desafio_pequeno() {
    let solutions = cap::solve("token-de-teste", 1, 16, 1);
    assert_eq!(solutions.len(), 1);
    assert!(cap::verify_solution(
        "token-de-teste",
        0,
        16,
        1,
        solutions[0]
    ));
}
