'use client'
import { Layout } from '@douyinfe/semi-ui'
import { AuthGuard } from './lib/auth-guard'
import ProtectedLayout from './lib/protected-layout'

const Home: React.FC = () => (
  <AuthGuard>
    <ProtectedLayout>
      <Layout>
        <iframe
          style={{
            borderWidth: 0,
          }}
          height="100%"
          src="https://biliup.github.io/biliup/docs/guide/changelog/"
        ></iframe>
      </Layout>
    </ProtectedLayout>
  </AuthGuard>
)

export default Home
