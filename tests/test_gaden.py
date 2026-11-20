import numpy as np

from gaden import control, neuron, pomc


def test_rest_is_quiet():
    r = neuron.simulate(0.0, t_end=100.0)
    assert r["spikes"] == 0 and r["v"].max() < -55


def test_moderate_current_fires():
    assert neuron.simulate(20.0, t_end=300.0)["spikes"] > 10


def test_high_current_depolarization_block():
    r = neuron.simulate(300.0, t_end=300.0)
    assert neuron.is_blocked(r) and r["h"][-1] < 0.1


def test_pomc_waveform():
    t = pomc.pulse_times_ms(3000.0)
    assert t[0] == 0 and abs(np.diff(t[:4]).mean() - 1000 / 30) < 1e-6
    assert list(pomc.burst_envelope([0, 499, 500, 1499, 1500])) == [
        True,
        True,
        False,
        False,
        True,
    ]
    assert abs(pomc.charge_per_phase_nC() - 3.75) < 1e-9
    assert pomc.duty_cycle() < 0.01


def test_linear_loop_stable():
    for ev in control.closed_loop_eigenvalues(control.Plant(), control.PI()):
        assert (ev.real < 0).all()


def test_closed_loop_meets_setpoints():
    c = control.metrics(control.simulate(closed=True))
    o = control.metrics(control.simulate(closed=False))
    assert c["frac_agrp_below_2hz"] > 0.95
    assert abs(c["pomc_mean_hz"] - 8) < 1.0
    assert o["agrp_max_hz"] > 20


def test_actuator_limits_respected():
    r = control.simulate(hunger_peak=60.0)
    assert r["u"][:, 0].max() <= 40.0 + 1e-9 and r["u"][:, 1].max() <= 25.0 + 1e-9
