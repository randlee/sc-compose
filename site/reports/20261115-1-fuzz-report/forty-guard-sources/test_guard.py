import unittest
from guard import isolated_env
class GuardTest(unittest.TestCase):
 def setUp(self):
  self.w='/tmp/campaign/workspace'; self.env={'HOME':'/tmp/campaign/home','BEADS_DIR':self.w+'/.beads','SC_COMPOSE_TEST_DOLT_PORT':'52345'}
 def test_allowed(self): self.assertEqual(isolated_env(self.env,self.w,52345),self.env)
 def test_shared_port(self):
  with self.assertRaises(ValueError): isolated_env(self.env,self.w,3308)
 def test_shared_beads(self):
  self.env['BEADS_DIR']='/Users/randlee/.beads'
  with self.assertRaises(ValueError): isolated_env(self.env,self.w,52345)
 def test_home(self):
  self.env['HOME']='/Users/randlee'
  with self.assertRaises(ValueError): isolated_env(self.env,self.w,52345)
 def test_port_mismatch(self):
  with self.assertRaises(ValueError): isolated_env(self.env,self.w,52346)
if __name__=='__main__': unittest.main()
